use std::collections::BTreeMap;
use std::path::Path;

use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::{Api, DynamicObject, ObjectMeta, Patch, PatchParams, PostParams},
    config::{KubeConfigOptions, Kubeconfig},
    core::GroupVersionKind,
    discovery::{Discovery, Scope},
    Client, Config, ResourceExt,
};
use tokio::process::Command;
use tracing::{error, info, warn};

/// Applies a per-workspace kustomize overlay generated from `base_path` inside the vcluster
/// identified by `kubeconfig`.
///
/// Uses `kubectl kustomize` (local file processing — no cluster access) to render the overlay,
/// then creates the target namespace and applies each resource via the kube-rs API.
pub async fn apply_ws_kustomization(
    ws_name: &str,
    repos: &[String],
    base_path: &Path,
    kubeconfig: &[u8],
) -> anyhow::Result<()> {
    let tmp = tempfile::tempdir()?;
    let overlay_dir = tmp.path().join("overlay");
    std::fs::create_dir_all(&overlay_dir)?;

    // Copy base into a sibling directory so overlay can reference it as ../base.
    let base_copy = tmp.path().join("base");
    copy_dir(base_path, &base_copy)?;

    let repos_json = serde_json::to_string(repos)?;

    // Build the patch body as a structured value; serde_yaml serialises it as a
    // block scalar (|) which is what kustomize expects for the patch field.
    let patch_body = serde_yaml::to_string(&serde_json::json!({
        "apiVersion": "v1",
        "kind": "ConfigMap",
        "metadata": { "name": "ws-config" },
        "data": { "WS_NAME": ws_name, "REPOS": repos_json },
    }))
    .map_err(|e| anyhow::anyhow!("failed to serialize patch body: {e}"))?;

    let kustomization = serde_yaml::to_string(&serde_json::json!({
        "apiVersion": "kustomize.config.k8s.io/v1beta1",
        "kind": "Kustomization",
        "resources": ["../base"],
        "namespace": format!("ws-{ws_name}"),
        "labels": [{
            "pairs": { "mahakam.io/managed": "true", "mahakam.io/ws": ws_name },
            "includeSelectors": false,
        }],
        "patches": [{
            "patch": patch_body,
            "target": { "kind": "ConfigMap", "name": "ws-config" },
        }],
    }))
    .map_err(|e| anyhow::anyhow!("failed to serialize kustomization: {e}"))?;

    std::fs::write(overlay_dir.join("kustomization.yaml"), &kustomization)?;

    // Build a kube client targeting the vcluster API server.
    let kubeconfig_text = std::str::from_utf8(kubeconfig)
        .map_err(|e| anyhow::anyhow!("kubeconfig is not valid UTF-8: {e}"))?;
    let parsed_kubeconfig = Kubeconfig::from_yaml(kubeconfig_text)
        .map_err(|e| anyhow::anyhow!("failed to parse vcluster kubeconfig: {e}"))?;
    let kube_config =
        Config::from_custom_kubeconfig(parsed_kubeconfig, &KubeConfigOptions::default())
            .await
            .map_err(|e| anyhow::anyhow!("failed to build vcluster client config: {e}"))?;
    let vcluster_client = Client::try_from(kube_config)
        .map_err(|e| anyhow::anyhow!("failed to create vcluster client: {e}"))?;

    // Create the target namespace inside the vcluster (it starts empty).
    let ns_name = format!("ws-{ws_name}");
    let namespaces: Api<Namespace> = Api::all(vcluster_client.clone());
    let ns = Namespace {
        metadata: ObjectMeta {
            name: Some(ns_name.clone()),
            labels: Some(BTreeMap::from([
                ("mahakam.io/managed".to_string(), "true".to_string()),
                ("mahakam.io/ws".to_string(), ws_name.to_string()),
            ])),
            ..Default::default()
        },
        ..Default::default()
    };
    match namespaces.create(&PostParams::default(), &ns).await {
        Ok(_) => info!(namespace = %ns_name, "created namespace in vcluster"),
        Err(kube::Error::Api(ref e)) if e.code == 409 => {
            info!(namespace = %ns_name, "namespace already exists in vcluster");
        }
        Err(e) => {
            return Err(anyhow::anyhow!(
                "failed to create namespace {ns_name} in vcluster: {e}"
            ))
        }
    }

    // Render the kustomize overlay to YAML (local file operation — no cluster access).
    info!(ws = %ws_name, overlay = %overlay_dir.display(), "rendering kustomize overlay");
    let build_output = Command::new("kubectl")
        .args(["kustomize", &overlay_dir.display().to_string()])
        .output()
        .await?;

    if !build_output.status.success() {
        let stderr = String::from_utf8_lossy(&build_output.stderr);
        error!(ws = %ws_name, "kubectl kustomize failed: {stderr}");
        anyhow::bail!("kubectl kustomize failed for ws {ws_name}: {stderr}");
    }

    let manifests_yaml = String::from_utf8(build_output.stdout)?;

    // Discover available API resources in the vcluster to resolve GVKs.
    let discovery = Discovery::new(vcluster_client.clone()).run().await?;

    // Apply each manifest document via server-side apply.
    for doc in manifests_yaml.split("\n---\n") {
        let doc = doc.trim();
        if doc.is_empty() {
            continue;
        }

        let json: serde_json::Value = serde_yaml::from_str(doc)
            .map_err(|e| anyhow::anyhow!("failed to parse kustomize manifest: {e}"))?;

        if json.is_null() {
            continue;
        }

        let api_version = json["apiVersion"].as_str().unwrap_or("").to_string();
        let kind = json["kind"].as_str().unwrap_or("").to_string();
        let (group, version) = match api_version.split_once('/') {
            Some((g, v)) => (g.to_string(), v.to_string()),
            None => (String::new(), api_version.clone()),
        };
        let gvk = GroupVersionKind::gvk(&group, &version, &kind);

        let dyn_obj: DynamicObject = serde_json::from_value(json)
            .map_err(|e| anyhow::anyhow!("failed to deserialize {kind}: {e}"))?;

        let Some((ar, caps)) = discovery.resolve_gvk(&gvk) else {
            warn!(ws = %ws_name, kind = %kind, "GVK not found in vcluster, skipping");
            continue;
        };

        let obj_ns = dyn_obj
            .namespace()
            .unwrap_or_else(|| format!("ws-{ws_name}"));
        let api: Api<DynamicObject> = match caps.scope {
            Scope::Namespaced => Api::namespaced_with(vcluster_client.clone(), &obj_ns, &ar),
            Scope::Cluster => Api::all_with(vcluster_client.clone(), &ar),
        };

        let name = dyn_obj.name_any();
        api.patch(
            &name,
            &PatchParams::apply("mahakam").force(),
            &Patch::Apply(&dyn_obj),
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to apply {kind}/{name}: {e}"))?;

        info!(ws = %ws_name, kind = %kind, name = %name, "applied resource in vcluster");
    }

    info!(ws = %ws_name, "kustomization applied inside vcluster");
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
