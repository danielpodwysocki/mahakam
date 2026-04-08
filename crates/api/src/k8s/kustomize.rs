use std::path::Path;

use tokio::process::Command;
use tracing::{error, info};

/// Applies a per-environment kustomize overlay generated from `base_path`.
///
/// Creates a temp directory with a generated overlay, then runs
/// `kubectl apply -k <tempdir>`.
pub async fn apply_env_kustomization(
    env_name: &str,
    repos: &[String],
    base_path: &Path,
) -> anyhow::Result<()> {
    let tmp = tempfile::tempdir()?;
    let overlay_dir = tmp.path();

    let repos_json = serde_json::to_string(repos)?;
    let base_abs = base_path
        .canonicalize()
        .unwrap_or_else(|_| base_path.to_path_buf());

    let kustomization = format!(
        r#"apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization
resources:
  - {base}
namespace: env-{name}
commonLabels:
  mahakam.io/managed: "true"
  mahakam.io/env: {name}
patches:
  - patch: |
      apiVersion: v1
      kind: ConfigMap
      metadata:
        name: env-config
      data:
        ENV_NAME: {name}
        REPOS: '{repos}'
    target:
      kind: ConfigMap
      name: env-config
"#,
        base = base_abs.display(),
        name = env_name,
        repos = repos_json,
    );

    std::fs::write(overlay_dir.join("kustomization.yaml"), &kustomization)?;

    info!(env = %env_name, overlay = %overlay_dir.display(), "applying kustomization");

    let status = Command::new("kubectl")
        .args(["apply", "-k", &overlay_dir.display().to_string()])
        .status()
        .await?;

    if !status.success() {
        error!(env = %env_name, "kubectl apply -k failed");
        anyhow::bail!("kubectl apply -k failed for env {env_name}");
    }

    info!(env = %env_name, "kustomization applied");
    Ok(())
}
