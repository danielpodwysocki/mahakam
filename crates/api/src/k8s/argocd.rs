use kube::{
    api::{Api, DeleteParams, DynamicObject, PostParams},
    discovery::ApiResource,
    Client,
};
use tokio::time::{sleep, timeout, Duration};
use tracing::{info, warn};

const APP_GROUP: &str = "argoproj.io";
const APP_VERSION: &str = "v1alpha1";
const APP_KIND: &str = "Application";
const APP_PLURAL: &str = "applications";
const CASCADE_FINALIZER: &str = "resources-finalizer.argocd.argoproj.io";

const HEALTH_WAIT_TIMEOUT_SECS: u64 = 600;
const POLL_INTERVAL_SECS: u64 = 5;

/// Parameters for the outer ArgoCD Application created per environment.
pub struct EnvApplicationSpec<'a> {
    pub env_name: &'a str,
    /// Git repository URL containing `chart/environment/`.
    pub repo_url: &'a str,
    /// Git revision (branch, tag, or SHA). "HEAD" tracks the default branch.
    pub repo_revision: &'a str,
    /// Namespace where ArgoCD is installed.
    pub argocd_namespace: &'a str,
    /// ArgoCD AppProject for all environment Applications.
    pub argocd_project: &'a str,
    /// vcluster Helm chart version to pin in the inner Application.
    pub vcluster_chart_version: &'a str,
}

fn application_api(client: &Client, namespace: &str) -> Api<DynamicObject> {
    let ar = ApiResource {
        group: APP_GROUP.into(),
        version: APP_VERSION.into(),
        api_version: format!("{APP_GROUP}/{APP_VERSION}"),
        kind: APP_KIND.into(),
        plural: APP_PLURAL.into(),
    };
    Api::namespaced_with(client.clone(), namespace, &ar)
}

/// Creates the outer ArgoCD Application for `env_name`.
///
/// The Application sources `chart/environment` from the mahakam git repository.
/// When ArgoCD syncs it, two resources are created in order via sync waves:
/// - wave -1: `Namespace env-{name}` (labeled for mahakam management)
/// - wave  0: `Application vcluster-{name}` (installs the vcluster Helm chart)
///
/// Both child resources carry `resources-finalizer.argocd.argoproj.io` so cascade
/// deletion unwinds everything in reverse-wave order when the outer App is deleted.
pub async fn create_env_application(
    client: &Client,
    spec: &EnvApplicationSpec<'_>,
) -> anyhow::Result<()> {
    let app_name = format!("env-{}", spec.env_name);

    let helm_values = format!(
        "envName: {env}\nargocdNamespace: {ns}\nargocdProject: {proj}\nvclusterChartVersion: {ver}\n",
        env = spec.env_name,
        ns = spec.argocd_namespace,
        proj = spec.argocd_project,
        ver = spec.vcluster_chart_version,
    );

    let app_json = serde_json::json!({
        "apiVersion": "argoproj.io/v1alpha1",
        "kind": "Application",
        "metadata": {
            "name": app_name,
            "namespace": spec.argocd_namespace,
            "finalizers": [CASCADE_FINALIZER],
        },
        "spec": {
            "project": spec.argocd_project,
            "source": {
                "repoURL": spec.repo_url,
                "path": "chart/environment",
                "targetRevision": spec.repo_revision,
                "helm": {
                    "values": helm_values,
                },
            },
            "destination": {
                "server": "https://kubernetes.default.svc",
                "namespace": spec.argocd_namespace,
            },
            "syncPolicy": {
                "automated": {
                    "prune": true,
                    "selfHeal": true,
                },
            },
        },
    });

    let obj: DynamicObject = serde_json::from_value(app_json)
        .map_err(|e| anyhow::anyhow!("failed to build Application object: {e}"))?;

    let api = application_api(client, spec.argocd_namespace);
    api.create(&PostParams::default(), &obj)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create Application {app_name}: {e}"))?;

    info!(env = %spec.env_name, app = %app_name, "ArgoCD Application created");
    Ok(())
}

/// Polls the outer Application and then the inner vcluster Application until
/// both are `Healthy` and `Synced`.
///
/// The outer App (`env-{name}`) becomes Healthy quickly once it has created the
/// namespace and the inner Application object, but the inner App
/// (`vcluster-{name}`) only reaches Healthy after the vcluster Helm chart has
/// fully installed. Waiting for both prevents connecting to the vcluster API
/// server before it is ready.
pub async fn wait_for_env_healthy(
    client: &Client,
    env_name: &str,
    argocd_namespace: &str,
) -> anyhow::Result<()> {
    let outer = format!("env-{env_name}");
    let inner = format!("vcluster-{env_name}");
    // Outer app manages namespace + inner Application object; must be Healthy+Synced.
    wait_for_application_healthy(client, &outer, env_name, argocd_namespace, true).await?;
    // vcluster mutates its own resources post-install, so ArgoCD perpetually shows
    // OutOfSync for the inner app. We only need it to be Healthy (pod running).
    wait_for_application_healthy(client, &inner, env_name, argocd_namespace, false).await
}

async fn wait_for_application_healthy(
    client: &Client,
    app_name: &str,
    env_name: &str,
    argocd_namespace: &str,
    require_synced: bool,
) -> anyhow::Result<()> {
    let api = application_api(client, argocd_namespace);

    info!(env = %env_name, app = %app_name, require_synced, "waiting for ArgoCD Application to be Healthy");

    timeout(Duration::from_secs(HEALTH_WAIT_TIMEOUT_SECS), async {
        loop {
            match api.get(app_name).await {
                Ok(obj) => {
                    let health = obj
                        .data
                        .pointer("/status/health/status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    let sync = obj
                        .data
                        .pointer("/status/sync/status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");

                    info!(env = %env_name, app = %app_name, health, sync, "ArgoCD Application status");

                    let synced_ok = !require_synced || sync == "Synced";
                    if health == "Healthy" && synced_ok {
                        return Ok::<(), anyhow::Error>(());
                    }

                    if health == "Degraded" {
                        let msg = obj
                            .data
                            .pointer("/status/conditions/0/message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("no message");
                        return Err(anyhow::anyhow!("Application {app_name} is Degraded: {msg}"));
                    }
                }
                Err(kube::Error::Api(ref e)) if e.code == 404 => {
                    warn!(env = %env_name, app = %app_name, "Application not yet found, retrying");
                }
                Err(e) => return Err(anyhow::anyhow!(e)),
            }
            sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
        }
    })
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "timed out after {}s waiting for Application {app_name} to be Healthy",
            HEALTH_WAIT_TIMEOUT_SECS
        )
    })?
}

/// Requests deletion of the outer Application and returns immediately.
///
/// ArgoCD's `resources-finalizer` handles cascade cleanup asynchronously:
/// it prunes the inner vcluster Application (and its Helm release) then removes
/// the namespace — unrolling everything in reverse-wave order without any
/// explicit kubectl or helm calls from mahakam.
///
/// The caller does not need to wait for cascade completion; ArgoCD will finish
/// in the background. Treating a 404 as success means calling this twice is safe.
pub async fn delete_env_application(
    client: &Client,
    env_name: &str,
    argocd_namespace: &str,
) -> anyhow::Result<()> {
    let app_name = format!("env-{env_name}");
    let api = application_api(client, argocd_namespace);

    match api.delete(&app_name, &DeleteParams::default()).await {
        Ok(_) => info!(env = %env_name, app = %app_name, "ArgoCD Application deletion requested"),
        Err(kube::Error::Api(ref e)) if e.code == 404 => {
            info!(env = %env_name, app = %app_name, "Application not found, nothing to delete");
        }
        Err(e) => {
            return Err(anyhow::anyhow!(
                "failed to delete Application {app_name}: {e}"
            ))
        }
    }

    Ok(())
}
