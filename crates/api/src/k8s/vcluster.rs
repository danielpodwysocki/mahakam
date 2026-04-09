use k8s_openapi::api::core::v1::Secret;
use kube::{api::Api, Client};
use tokio::time::{sleep, timeout, Duration};
use tracing::{info, warn};

// vcluster chart (0.33.x) creates a secret named "vc-vcluster-{name}" in "env-{name}",
// where "vcluster-{name}" is the Helm release name set by the inner ArgoCD Application.
// Key "config" holds the kubeconfig YAML with server "https://localhost:8443"; it is
// patched to the in-cluster service address before use.
const KUBECONFIG_SECRET_PREFIX: &str = "vc-vcluster-";
const KUBECONFIG_SECRET_KEY: &str = "config";
const KUBECONFIG_LOCALHOST: &str = "https://localhost:8443";

const KUBECONFIG_WAIT_TIMEOUT_SECS: u64 = 300;
const KUBECONFIG_POLL_INTERVAL_SECS: u64 = 5;

/// Polls for the vcluster kubeconfig secret until it appears, then returns its bytes.
///
/// The secret is created by the vcluster chart regardless of whether it was installed
/// directly or via ArgoCD. Times out after 300 seconds.
pub async fn wait_for_vcluster_kubeconfig(
    client: &Client,
    env_name: &str,
) -> anyhow::Result<Vec<u8>> {
    let namespace = format!("env-{env_name}");
    let secret_name = format!("{KUBECONFIG_SECRET_PREFIX}{env_name}");
    let secrets: Api<Secret> = Api::namespaced(client.clone(), &namespace);

    info!(secret = %secret_name, namespace = %namespace, "waiting for vcluster kubeconfig secret");

    timeout(Duration::from_secs(KUBECONFIG_WAIT_TIMEOUT_SECS), async {
        loop {
            match secrets.get(&secret_name).await {
                Ok(secret) => {
                    let data = secret
                        .data
                        .ok_or_else(|| anyhow::anyhow!("secret {} has no data", secret_name))?;
                    let raw = data
                        .get(KUBECONFIG_SECRET_KEY)
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "secret {} missing key '{}'",
                                secret_name,
                                KUBECONFIG_SECRET_KEY
                            )
                        })?
                        .0
                        .clone();

                    let in_cluster_server =
                        format!("https://vcluster-{env_name}.env-{env_name}:443");
                    let kubeconfig = String::from_utf8(raw)
                        .map_err(|e| anyhow::anyhow!("kubeconfig is not valid UTF-8: {}", e))?
                        .replace(KUBECONFIG_LOCALHOST, &in_cluster_server)
                        .into_bytes();

                    info!(secret = %secret_name, server = %in_cluster_server, "kubeconfig ready");
                    return Ok::<Vec<u8>, anyhow::Error>(kubeconfig);
                }
                Err(kube::Error::Api(err)) if err.code == 404 => {
                    warn!(secret = %secret_name, "kubeconfig secret not yet available, retrying");
                    sleep(Duration::from_secs(KUBECONFIG_POLL_INTERVAL_SECS)).await;
                }
                Err(e) => return Err(anyhow::anyhow!(e)),
            }
        }
    })
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "timed out after {}s waiting for vcluster kubeconfig secret {}",
            KUBECONFIG_WAIT_TIMEOUT_SECS,
            secret_name
        )
    })?
}
