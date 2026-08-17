//! A thin client reading a target's `/healthz/config` — the
//! `shikumi::hotswap::ConfigSyncProof` (`theory/CALHA.md` §6.3) a hot-swap-
//! aware target exposes. Ported SHAPE, not imported TYPE, from
//! `breathe-provider`'s `ConfigReload`-gated-write pattern; the mirror types
//! below are `calha`'s own -- this crate has no compile-time dependency on
//! `shikumi`, which is deliberate: a client should be able to speak a wire
//! format without linking the server's crate.
//!
//! CORRECTED 2026-08-11. The claim used to be that these matched "the wire
//! shape shikumi will serve once M1/M2 land". They did not, and could not be
//! checked: `shikumi::hotswap::ConfigSyncProof` derived no `Serialize`, so
//! there was NO wire format to match. In the gap this mirror lost the `free`
//! hash entirely.
//!
//! shikumi now ships `ConfigSyncProofWire` (hex hashes, camelCase,
//! epoch-seconds). These types mirror THAT, field for field. The rule for
//! keeping them honest is a round-trip test against a payload shaped like the
//! producer's -- a mirror with no producer is a guess wearing a type.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigWatermark {
    pub full: String,
    pub restart_required: String,
    /// The `Free`-class half — hash of the fields a running process may swap
    /// in memory.
    ///
    /// ADDED 2026-08-11, and its absence was not cosmetic. This mirror carried
    /// two hashes where `shikumi::hotswap::ConfigWatermark` has three, so a
    /// deserialize of a real payload would have failed on an unknown field —
    /// and, worse, calha could not answer "did a hot-swappable knob move?",
    /// which is the entire reason the watermark is SPLIT.
    ///
    /// It drifted undetected because there was nothing to drift from: shikumi
    /// derived no `Serialize` at all, so no wire format existed. It does now
    /// (`shikumi::hotswap::ConfigSyncProofWire`), and these fields match it
    /// exactly — hex hashes, camelCase, epoch-seconds timestamp.
    pub free: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSyncProof {
    pub generation: u64,
    pub watermark: ConfigWatermark,
    /// Unix epoch (secs) this proof was observed at the target.
    pub observed_at_epoch: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum WatermarkError {
    #[error("http request to {url} failed: {source}")]
    Request {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("target at {url} returned non-success status {status}")]
    Status {
        url: String,
        status: reqwest::StatusCode,
    },
}

/// Fetches a target's current [`ConfigSyncProof`] from its `/healthz/config`
/// endpoint. Read-only, no retry policy of its own -- the caller (the
/// reconcile loop) owns backoff via its own requeue interval.
pub async fn fetch_sync_proof(
    client: &reqwest::Client,
    target_base_url: &str,
) -> Result<ConfigSyncProof, WatermarkError> {
    let url = format!("{target_base_url}/healthz/config");
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|source| WatermarkError::Request {
            url: url.clone(),
            source,
        })?;

    if !resp.status().is_success() {
        return Err(WatermarkError::Status {
            url,
            status: resp.status(),
        });
    }

    resp.json::<ConfigSyncProof>()
        .await
        .map_err(|source| WatermarkError::Request { url, source })
}

#[cfg(test)]
mod wire_parity_tests {
    use super::*;

    /// A payload shaped exactly as `shikumi::hotswap::ConfigSyncProofWire`
    /// serializes it. Hand-written on purpose: calha does not link shikumi, so
    /// this literal IS the contract, and it is the thing that goes stale if the
    /// producer moves.
    const PRODUCER_PAYLOAD: &str = r#"{
      "generation": 7,
      "watermark": {
        "full": "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
        "restartRequired": "2f0b1a0e0f1e2d3c4b5a69788796a5b4c3d2e1f00f1e2d3c4b5a69788796a5b4",
        "free": "1e2d3c4b5a69788796a5b4c3d2e1f00f1e2d3c4b5a69788796a5b4c3d2e1f00f"
      },
      "observedAtEpoch": 1700000000
    }"#;

    /// The whole point of the fix: a real producer payload must deserialize.
    /// Before `free` was added this failed, and nothing would have caught it
    /// until calha polled a live target.
    #[test]
    fn a_producer_shaped_payload_deserializes() {
        let p: ConfigSyncProof =
            serde_json::from_str(PRODUCER_PAYLOAD).expect("producer payload must deserialize");
        assert_eq!(p.generation, 7);
        assert_eq!(p.observed_at_epoch, 1_700_000_000);
        assert_eq!(p.watermark.free.len(), 64);
    }

    /// The three hashes are distinct roles, not decoration — collapsing any two
    /// would make "did a hot-swappable knob move?" unanswerable.
    #[test]
    fn all_three_watermark_halves_are_present_and_distinct() {
        let p: ConfigSyncProof = serde_json::from_str(PRODUCER_PAYLOAD).unwrap();
        let w = &p.watermark;
        assert_ne!(w.full, w.restart_required);
        assert_ne!(w.restart_required, w.free);
        assert_ne!(w.full, w.free);
    }

    /// Round-trip: what calha emits must be what calha can read.
    #[test]
    fn the_mirror_round_trips() {
        let p: ConfigSyncProof = serde_json::from_str(PRODUCER_PAYLOAD).unwrap();
        let back: ConfigSyncProof =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(p, back);
    }
}
