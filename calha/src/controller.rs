//! The `CalhaPolicy` reconciler. Mirrors `breathe-controller`'s own
//! `gen_controller!` idiom (`breathe-controller/src/main.rs:63-72`):
//! `predicate_filter(predicates::generation)` so a status-only self-patch
//! never re-triggers the watch, `Action::requeue` drives the periodic tick.
//!
//! `theory/CALHA.md` §14.1 chose ONE crate with internal module boundaries
//! over a four-crate split; this module owns the wiring, `runtime::plan_tick`
//! owns the decision, `watermark` owns the read.

use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    api::{Api, Patch, PatchParams},
    runtime::{controller::Action, predicates, reflector, watcher, Controller, WatchStreamExt},
    Client, ResourceExt,
};
use serde_json::json;
use tracing::{error, info, warn};

use crate::crd::{CalhaPolicy, CalhaPolicyStatus};
use crate::runtime::{plan_tick, TickOutcome};
use crate::watermark::{fetch_sync_proof, WatermarkError};

const DEFAULT_REQUEUE_SECS: u64 = 30;
const DEFAULT_ERROR_REQUEUE_SECS: u64 = 15;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("kube: {0}")]
    Kube(#[from] kube::Error),
    #[error("watermark fetch: {0}")]
    Watermark(#[from] WatermarkError),
}

pub struct Ctx {
    pub client: Client,
    pub http: reqwest::Client,
    pub requeue: Duration,
    /// True when the fleet-wide freeze switch is engaged (mirrors breathe's
    /// `!writeEnabled` — `outorga`'s `frozen` two-key input).
    pub frozen: bool,
}

async fn reconcile(policy: Arc<CalhaPolicy>, ctx: Arc<Ctx>) -> Result<Action, Error> {
    let ns = policy.spec.target_ref.namespace.clone();
    let name = policy.name_any();
    info!(policy = %name, namespace = %ns, "reconciling CalhaPolicy");

    let target_base_url = format!(
        "http://{}.{}.svc:{}",
        policy.spec.target_ref.deployment_name, ns, policy.spec.target_ref.healthz_port
    );

    let proof = match fetch_sync_proof(&ctx.http, &target_base_url).await {
        Ok(p) => p,
        Err(e) => {
            warn!(policy = %name, error = %e, "failed to read target sync proof, will retry");
            return Ok(Action::requeue(Duration::from_secs(DEFAULT_ERROR_REQUEUE_SECS)));
        }
    };

    let last_applied = policy.status.as_ref().and_then(|s| s.last_applied_watermark.as_deref());
    let now = chrono::Utc::now().timestamp();

    let outcome = plan_tick(&policy.spec.promotion, last_applied, &proof, now, ctx.frozen);

    let deployments: Api<Deployment> = Api::namespaced(ctx.client.clone(), &ns);
    let policies: Api<CalhaPolicy> = Api::namespaced(ctx.client.clone(), &ns);

    let new_status = match &outcome {
        TickOutcome::NoChange => CalhaPolicyStatus {
            wants_restart: Some(false),
            last_applied_watermark: last_applied.map(str::to_string),
            last_error: None,
        },
        TickOutcome::ObservedChangeShadowed(reason) => {
            info!(policy = %name, reason = ?reason, "restart-required watermark changed, held in shadow");
            CalhaPolicyStatus {
                wants_restart: Some(true),
                last_applied_watermark: last_applied.map(str::to_string),
                last_error: None,
            }
        }
        TickOutcome::Apply { new_watermark } => {
            info!(policy = %name, watermark = %new_watermark, "applying -- patching pod-template-hash annotation to drive a rolling update");
            // Per theory/CALHA.md SS6.5: patches ONLY the pod-template-hash
            // annotation on the target Deployment, driving a normal rolling
            // update. Never touches the ConfigMap (Flux already owns that write).
            let patch = json!({
                "spec": {
                    "template": {
                        "metadata": {
                            "annotations": {
                                "calha.pleme.io/config-watermark": new_watermark,
                            }
                        }
                    }
                }
            });
            if let Err(e) = deployments
                .patch(
                    &policy.spec.target_ref.deployment_name,
                    &PatchParams::apply("calha"),
                    &Patch::Merge(&patch),
                )
                .await
            {
                error!(policy = %name, error = %e, "failed to patch target Deployment");
                CalhaPolicyStatus {
                    wants_restart: Some(true),
                    last_applied_watermark: last_applied.map(str::to_string),
                    last_error: Some(e.to_string()),
                }
            } else {
                CalhaPolicyStatus {
                    wants_restart: Some(false),
                    last_applied_watermark: Some(new_watermark.clone()),
                    last_error: None,
                }
            }
        }
    };

    let status_patch = json!({ "status": new_status });
    policies
        .patch_status(&name, &PatchParams::apply("calha"), &Patch::Merge(&status_patch))
        .await?;

    Ok(Action::requeue(ctx.requeue))
}

fn error_policy(_policy: Arc<CalhaPolicy>, err: &Error, _ctx: Arc<Ctx>) -> Action {
    error!(error = %err, "reconcile error");
    Action::requeue(Duration::from_secs(DEFAULT_ERROR_REQUEUE_SECS))
}

/// Runs the `CalhaPolicy` controller to completion (i.e. forever, until the
/// stream errors out). `predicate_filter(predicates::generation)` -- the
/// exact idiom at `breathe-controller/src/main.rs:69-70` -- means our own
/// status patch above never re-triggers this reconcile.
pub async fn run(client: Client, frozen: bool) {
    let policies: Api<CalhaPolicy> = Api::all(client.clone());
    let ctx = Arc::new(Ctx {
        client: client.clone(),
        http: reqwest::Client::new(),
        requeue: Duration::from_secs(DEFAULT_REQUEUE_SECS),
        frozen,
    });

    let (reader, writer) = reflector::store();
    let stream = watcher(policies, watcher::Config::default())
        .reflect(writer)
        .applied_objects()
        .predicate_filter(predicates::generation);

    Controller::for_stream(stream, reader)
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok(o) => info!(?o, "reconciled"),
                Err(e) => error!(error = %e, "reconcile failed"),
            }
        })
        .await;
}
