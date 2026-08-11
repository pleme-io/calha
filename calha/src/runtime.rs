//! `plan_tick` — the pure, zero-I/O decision core. Reads a target's watermark
//! observation, compares it to the last-observed value, and decides whether
//! (and how) `calha` should act this tick. No k8s client, no HTTP client, no
//! clock read inside the decision itself (mirrors `outorga::PromotionPolicy::decide`'s
//! own purity) — every reachable outcome is a table-driven unit test.

use outorga::{Observation, PromotionDecision};

use crate::crd::CalhaPromotionPolicy;
use crate::watermark::ConfigSyncProof;

/// One tick's decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TickOutcome {
    /// The target's restart-required watermark is unchanged since last tick
    /// (or this is the first tick) — nothing to do.
    NoChange,
    /// The watermark changed, but the promotion policy holds this tick in
    /// shadow (compute-and-report only). Carries the typed reason.
    ObservedChangeShadowed(outorga::ShadowReason),
    /// The watermark changed AND the promotion policy authorizes a real
    /// apply — patch the target's pod-template-hash annotation.
    Apply { new_watermark: String },
}

/// A minimal `outorga::Observation` impl over a [`ConfigSyncProof`]. `calha`
/// fetches the proof fresh every tick (`watermark::fetch_sync_proof`), and
/// the caller (`controller::reconcile`) already short-circuits on a fetch
/// failure before `plan_tick` ever runs — so there is no independent
/// "is this read too old" signal to layer in here on top of that. `stale()`
/// and `conflict()` are honestly `false`: `ready_since()` carries the ONLY
/// age-relevant fact `outorga`'s confirm gate needs (since when has the
/// watermark itself been continuously this value), which is a property of
/// the *config*, not of the read.
pub struct WatermarkObservation<'a> {
    pub proof: &'a ConfigSyncProof,
    pub operator_confirmed: bool,
}

impl Observation for WatermarkObservation<'_> {
    fn ready(&self) -> bool {
        true
    }

    fn stale(&self) -> bool {
        false
    }

    fn conflict(&self) -> bool {
        false
    }

    fn ready_since(&self) -> Option<i64> {
        Some(self.proof.observed_at_epoch)
    }

    fn operator_confirmed(&self) -> bool {
        self.operator_confirmed
    }
}

/// The pure decision function. `last_applied` is `None` on the very first
/// tick for a target (nothing to compare against yet).
#[must_use]
pub fn plan_tick(
    policy: &CalhaPromotionPolicy,
    last_applied_watermark: Option<&str>,
    proof: &ConfigSyncProof,
    now_epoch: i64,
    frozen: bool,
) -> TickOutcome {
    let current = proof.watermark.restart_required.as_str();

    if last_applied_watermark == Some(current) {
        return TickOutcome::NoChange;
    }

    let obs = WatermarkObservation {
        proof,
        operator_confirmed: false,
    };

    match policy.to_outorga().decide(&obs, now_epoch, frozen) {
        PromotionDecision::Apply => TickOutcome::Apply {
            new_watermark: current.to_string(),
        },
        PromotionDecision::Shadow(reason) => TickOutcome::ObservedChangeShadowed(reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crd::{CalhaPromotionMode, CalhaPromotionPolicy};
    use crate::watermark::ConfigWatermark;

    fn proof(restart_required: &str, observed_at_epoch: i64) -> ConfigSyncProof {
        ConfigSyncProof {
            generation: 1,
            watermark: ConfigWatermark {
                full: "full-hash".to_string(),
                restart_required: restart_required.to_string(),
                // plan_tick compares ONLY restart_required — a Free-class edit
                // must not provoke a restart — so this half is deliberately
                // constant across these fixtures. Its presence is what makes
                // that indifference an assertion rather than an omission.
                free: "free-hash".to_string(),
            },
            observed_at_epoch,
        }
    }

    #[test]
    fn no_change_when_watermark_matches_last_applied() {
        let policy = CalhaPromotionPolicy {
            mode: CalhaPromotionMode::Effect,
            confirm_after_secs: 0,
        };
        let p = proof("abc", 1000);
        let outcome = plan_tick(&policy, Some("abc"), &p, 1000, false);
        assert_eq!(outcome, TickOutcome::NoChange);
    }

    #[test]
    fn first_tick_with_no_prior_watermark_is_a_change() {
        let policy = CalhaPromotionPolicy {
            mode: CalhaPromotionMode::Effect,
            confirm_after_secs: 0,
        };
        let p = proof("abc", 1000);
        let outcome = plan_tick(&policy, None, &p, 1000, false);
        assert_eq!(
            outcome,
            TickOutcome::Apply {
                new_watermark: "abc".to_string()
            }
        );
    }

    #[test]
    fn frozen_shadows_even_in_effect_mode() {
        let policy = CalhaPromotionPolicy {
            mode: CalhaPromotionMode::Effect,
            confirm_after_secs: 0,
        };
        let p = proof("xyz", 1000);
        let outcome = plan_tick(&policy, Some("abc"), &p, 1000, true);
        assert_eq!(
            outcome,
            TickOutcome::ObservedChangeShadowed(outorga::ShadowReason::Frozen)
        );
    }

    #[test]
    fn shadow_mode_never_applies() {
        let policy = CalhaPromotionPolicy {
            mode: CalhaPromotionMode::Shadow,
            confirm_after_secs: 0,
        };
        let p = proof("xyz", 1000);
        let outcome = plan_tick(&policy, Some("abc"), &p, 1000, false);
        assert_eq!(
            outcome,
            TickOutcome::ObservedChangeShadowed(outorga::ShadowReason::ModeShadow)
        );
    }

    #[test]
    fn shadow_confirm_effect_waits_out_the_window() {
        let policy = CalhaPromotionPolicy {
            mode: CalhaPromotionMode::ShadowConfirmEffect,
            confirm_after_secs: 1800,
        };
        let p = proof("xyz", 1000);
        // Watermark just changed at t=1000; observed_at_epoch == ready_since == 1000.
        let outcome = plan_tick(&policy, Some("abc"), &p, 1000, false);
        assert_eq!(
            outcome,
            TickOutcome::ObservedChangeShadowed(outorga::ShadowReason::ConfirmPending {
                held_secs: 0,
                need_secs: 1800,
            })
        );
    }

    #[test]
    fn shadow_confirm_effect_applies_once_window_elapses() {
        let policy = CalhaPromotionPolicy {
            mode: CalhaPromotionMode::ShadowConfirmEffect,
            confirm_after_secs: 1800,
        };
        let p = proof("xyz", 1000);
        let outcome = plan_tick(&policy, Some("abc"), &p, 1000 + 1800, false);
        assert_eq!(
            outcome,
            TickOutcome::Apply {
                new_watermark: "xyz".to_string()
            }
        );
    }

    #[test]
    fn confirm_window_survives_a_ready_since_far_in_the_past() {
        // A long-ready watermark (observed_at_epoch far before now) must
        // still promote once its OWN confirm_after_secs elapses, regardless
        // of how large the gap between observed_at_epoch and now is. This is
        // the regression test for the bug this design fixed: an earlier
        // draft's WatermarkObservation::stale() derived staleness from
        // `now - observed_at_epoch` using the SAME field ready_since() reads,
        // which made any confirm_after_secs window longer than the staleness
        // threshold permanently unsatisfiable (the observation looked
        // "stale" before the confirm gate could ever pass).
        let policy = CalhaPromotionPolicy {
            mode: CalhaPromotionMode::ShadowConfirmEffect,
            confirm_after_secs: 60,
        };
        let p = proof("xyz", 0);
        let outcome = plan_tick(&policy, Some("abc"), &p, 1_000_000, false);
        assert_eq!(
            outcome,
            TickOutcome::Apply {
                new_watermark: "xyz".to_string()
            }
        );
    }
}
