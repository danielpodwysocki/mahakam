use tokio::process::Command;
use tracing::{error, info};

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
