//! A thin client reading a target's `/healthz/config` — the
//! `shikumi::hotswap::ConfigSyncProof` (`theory/CALHA.md` §6.3) a hot-swap-
//! aware target exposes. Ported SHAPE, not imported TYPE, from
//! `breathe-provider`'s `ConfigReload`-gated-write pattern; the mirror types
//! below are `calha`'s own, matching the wire shape `shikumi::hotswap` will
//! serve once M1/M2 land (`theory/CALHA.md` §10) -- this crate has no
//! compile-time dependency on `shikumi`.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigWatermark {
    pub full: String,
    pub restart_required: String,
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
    Status { url: String, status: reqwest::StatusCode },
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
        .map_err(|source| WatermarkError::Request { url: url.clone(), source })?;

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
