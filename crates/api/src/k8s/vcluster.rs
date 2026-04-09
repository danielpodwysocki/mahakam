use k8s_openapi::api::core::v1::Secret;
use kube::{api::Api, Client};
use tokio::process::Command;
use tokio::time::{sleep, timeout, Duration};
use tracing::{error, info, warn};

// vcluster chart (0.33.x) creates a secret named "vc-vcluster-{name}" in "env-{name}",
// where "vcluster-{name}" is the helm release name. Key "config" holds the kubeconfig YAML.
// The kubeconfig server address is "https://localhost:8443" (designed for port-forwarding);
// it is patched to the in-cluster service address before use.
const KUBECONFIG_SECRET_PREFIX: &str = "vc-vcluster-";
const KUBECONFIG_SECRET_KEY: &str = "config";
const KUBECONFIG_LOCALHOST: &str = "https://localhost:8443";

const KUBECONFIG_WAIT_TIMEOUT_SECS: u64 = 300;
const KUBECONFIG_POLL_INTERVAL_SECS: u64 = 5;

/// Installs a vcluster into `env-<name>` using helm.
pub async fn install_vcluster(env_name: &str) -> anyhow::Result<()> {
    let release = format!("vcluster-{env_name}");
    let namespace = format!("env-{env_name}");

    info!(release = %release, namespace = %namespace, "installing vcluster");

    let status = Command::new("helm")
        .args([
            "upgrade",
            "--install",
            &release,
            "vcluster",
            "--repo",
            "https://charts.loft.sh",
            "--namespace",
            &namespace,
            "--wait",
            "--timeout",
            "5m",
        ])
        .status()
        .await?;

    if !status.success() {
        error!(release = %release, "helm install vcluster failed");
        anyhow::bail!("helm install vcluster-{env_name} failed");
    }

    info!(release = %release, "vcluster installed");
    Ok(())
}

/// Polls for the vcluster kubeconfig secret `vc-<name>` until it appears, then returns its bytes.
///
/// Times out after 300 seconds.
pub async fn wait_for_vcluster_kubeconfig(
    client: &Client,
    env_name: &str,
) -> anyhow::Result<Vec<u8>> {
    let namespace = format!("env-{env_name}");
    let secret_name = format!("{KUBECONFIG_SECRET_PREFIX}{env_name}");
    let secrets: Api<Secret> = Api::namespaced(client.clone(), &namespace);

    info!(secret = %secret_name, namespace = %namespace, "waiting for vcluster kubeconfig secret");

    timeout(
        Duration::from_secs(KUBECONFIG_WAIT_TIMEOUT_SECS),
        async {
            loop {
                match secrets.get(&secret_name).await {
                    Ok(secret) => {
                        let data = secret.data.ok_or_else(|| {
                            anyhow::anyhow!("secret {} has no data", secret_name)
                        })?;
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

                        // The kubeconfig server address is "localhost:8443" (port-forward
                        // convention). Replace it with the in-cluster service address so
                        // kubectl can reach the vcluster API server from within the pod.
                        let in_cluster_server = format!(
                            "https://vcluster-{env_name}.env-{env_name}:443"
                        );
                        let kubeconfig = String::from_utf8(raw)
                            .map_err(|e| anyhow::anyhow!("kubeconfig is not valid UTF-8: {}", e))?
                            .replace(KUBECONFIG_LOCALHOST, &in_cluster_server)
                            .into_bytes();

                        info!(secret = %secret_name, server = %in_cluster_server, "kubeconfig secret ready");
                        return Ok::<Vec<u8>, anyhow::Error>(kubeconfig);
                    }
                    Err(kube::Error::Api(err)) if err.code == 404 => {
                        warn!(secret = %secret_name, "kubeconfig secret not yet available, retrying");
                        sleep(Duration::from_secs(KUBECONFIG_POLL_INTERVAL_SECS)).await;
                    }
                    Err(e) => return Err(anyhow::anyhow!(e)),
                }
            }
        },
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "timed out after {}s waiting for vcluster kubeconfig secret {}",
            KUBECONFIG_WAIT_TIMEOUT_SECS,
            secret_name
        )
    })?
}

/// Uninstalls the vcluster for `env-<name>`. Ignores not-found errors.
pub async fn uninstall_vcluster(env_name: &str) -> anyhow::Result<()> {
    let release = format!("vcluster-{env_name}");
    let namespace = format!("env-{env_name}");

    info!(release = %release, "uninstalling vcluster");

    let status = Command::new("helm")
        .args(["uninstall", &release, "--namespace", &namespace])
        .status()
        .await?;

    // Non-zero exit for "not found" is acceptable; log and continue.
    if !status.success() {
        info!(release = %release, "helm uninstall returned non-zero (may be not-found), continuing");
    }

    Ok(())
}
