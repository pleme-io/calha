//! The `CalhaPolicy` CRD (group `calha.pleme.io/v1alpha1`) — one declaration
//! per target Deployment whose restart-required config half `calha` drives
//! through a rolling update. See `theory/CALHA.md` §5.2/§6.5 for the design.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Which Deployment + which ConfigMap a `CalhaPolicy` watches.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TargetRef {
    /// The Deployment `calha` patches (pod-template-hash annotation only).
    pub deployment_name: String,
    /// The namespace both the Deployment and the target's `/healthz/config`
    /// endpoint live in.
    pub namespace: String,
    /// The target's own `/healthz/config` port (serving a `ConfigSyncProof`,
    /// per `theory/CALHA.md` §6.3's `sync_proof()`).
    #[serde(default = "default_healthz_port")]
    pub healthz_port: u16,
}

fn default_healthz_port() -> u16 {
    9090
}

/// The only legal value today (`theory/CALHA.md` §5.2). `calha` may only act
/// on config whose resolved value came from the DISCOVERED tier — a fact
/// that was never committed to git in the first place, so patching it is not
/// a GitOps violation. `File`/`Env`/`Default`-tier changes flow through Flux,
/// full stop; `calha` has no code path that can touch them. Widening this to
/// a second arm is a real, reviewable decision (a closed-enum diff), not
/// something this design gives `calha` a path to do quietly.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum TierScope {
    #[default]
    Discovered,
}

/// A local, `JsonSchema`-bearing mirror of `outorga::PromotionPolicy` --
/// required because `outorga::PromotionPolicy`/`PromotionMode` deliberately
/// carry no `schemars::JsonSchema` impl (a pure-Rust decision type, not a
/// CRD-surface type). Mirrors `breathe-crd`'s own precedent exactly
/// (`breathe-crd/src/lib.rs:382-386` does the same for the same reason).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CalhaPromotionPolicy {
    pub mode: CalhaPromotionMode,
    #[serde(default = "default_confirm_after_secs")]
    pub confirm_after_secs: u64,
}

fn default_confirm_after_secs() -> u64 {
    outorga::DEFAULT_CONFIRM_AFTER_SECS
}

impl Default for CalhaPromotionPolicy {
    fn default() -> Self {
        Self {
            mode: CalhaPromotionMode::default(),
            confirm_after_secs: outorga::DEFAULT_CONFIRM_AFTER_SECS,
        }
    }
}

impl CalhaPromotionPolicy {
    /// The `outorga`-side mirror of this policy (identical variant set,
    /// identical window). This type stays distinct because it needs
    /// `JsonSchema` for the CRD surface, which `outorga::PromotionPolicy`
    /// deliberately does not carry (it has no k8s dependency at all).
    #[must_use]
    pub fn to_outorga(self) -> outorga::PromotionPolicy {
        outorga::PromotionPolicy::new(self.mode.to_outorga()).confirm_after(self.confirm_after_secs)
    }
}

/// Mirrors `outorga::PromotionMode` field-for-field. See
/// [`CalhaPromotionPolicy`]'s doc comment for why this is a distinct type.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum CalhaPromotionMode {
    Shadow,
    Effect,
    #[default]
    ShadowConfirmEffect,
    Suspended,
}

impl CalhaPromotionMode {
    #[must_use]
    pub fn to_outorga(self) -> outorga::PromotionMode {
        match self {
            Self::Shadow => outorga::PromotionMode::Shadow,
            Self::Effect => outorga::PromotionMode::Effect,
            Self::ShadowConfirmEffect => outorga::PromotionMode::ShadowConfirmEffect,
            Self::Suspended => outorga::PromotionMode::Suspended,
        }
    }
}

/// `calha.pleme.io/v1alpha1` `CalhaPolicy` — per §17 of CALHA.md's corrections
/// ledger, the Kind name matches the fleet's real convention (a repo with its
/// own minted domain word reuses that word as the Kind, mirroring
/// `breathe-crd`'s `Densa`/`QuinhaoPool` and `ensaio`'s `Ensaio`).
#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "calha.pleme.io",
    version = "v1alpha1",
    kind = "CalhaPolicy",
    namespaced,
    status = "CalhaPolicyStatus",
    printcolumn = r#"{"name":"Wants Restart","type":"boolean","jsonPath":".status.wantsRestart"}"#,
    printcolumn = r#"{"name":"Last Applied Watermark","type":"string","jsonPath":".status.lastAppliedWatermark"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct CalhaPolicySpec {
    pub target_ref: TargetRef,
    #[serde(default)]
    pub promotion: CalhaPromotionPolicy,
    #[serde(default)]
    pub tier_scope: TierScope,
}

/// Reconciler-owned status. `wants_restart` and `last_applied_watermark` are
/// the queryable convergence facts an operator (or `kubectl get calhapolicy`)
/// reads to answer "is this target on the config it should be on".
#[derive(Serialize, Deserialize, Clone, Debug, Default, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CalhaPolicyStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wants_restart: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_applied_watermark: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}
