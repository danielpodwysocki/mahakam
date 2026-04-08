use std::path::Path;

use tokio::process::Command;
use tracing::{error, info};

/// Applies a per-environment kustomize overlay generated from `base_path`.
///
/// Copies the base into the temp directory so kustomize can reference it
/// with a relative path (kustomize does not accept absolute paths).
/// Then runs `kubectl apply -k <tempdir/overlay>`.
pub async fn apply_env_kustomization(
    env_name: &str,
    repos: &[String],
    base_path: &Path,
) -> anyhow::Result<()> {
    let tmp = tempfile::tempdir()?;
    let overlay_dir = tmp.path().join("overlay");
    std::fs::create_dir_all(&overlay_dir)?;

    // Copy base into a sibling directory so overlay can reference it as ../base.
    let base_copy = tmp.path().join("base");
    copy_dir(base_path, &base_copy)?;

    let repos_json = serde_json::to_string(repos)?;

    let kustomization = format!(
        r#"apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization
resources:
  - ../base
namespace: env-{name}
labels:
  - pairs:
      mahakam.io/managed: "true"
      mahakam.io/env: {name}
    includeSelectors: false
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

/// Recursively copies a directory tree from `src` to `dst`.
fn copy_dir(src: &Path, dst: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}
