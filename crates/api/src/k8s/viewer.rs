use std::collections::{BTreeMap, HashMap};
use std::process::Stdio;

use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{
    Container, ContainerPort, EnvVar, HostPathVolumeSource, PodSpec, PodTemplateSpec,
    SecurityContext, Service, ServicePort, ServiceSpec, Volume, VolumeMount,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::{
    api::{Api, DeleteParams, DynamicObject, ListParams, ObjectMeta, PostParams},
    discovery::ApiResource,
    Client,
};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{info, warn};

use shared::repositories::workspace::Viewer;

const SELECTOR_LABEL: &str = "mahakam.io/viewer-instance";
const LABEL_VIEWER: &str = "mahakam.io/viewer";
const LABEL_WS_NAME: &str = "mahakam.io/ws-name";
const ANN_VIEWER_DISPLAY_NAME: &str = "mahakam.io/viewer-display-name";
const ANN_VIEWER_PATH: &str = "mahakam.io/viewer-path";
const ROUTE_NAMESPACE: &str = "mahakam-system";
const GATEWAY_NAME: &str = "mahakam";

const HTTPROUTE_GROUP: &str = "gateway.networking.k8s.io";
const HTTPROUTE_VERSION: &str = "v1";
const HTTPROUTE_KIND: &str = "HTTPRoute";
const HTTPROUTE_PLURAL: &str = "httproutes";

/// All the information needed to spawn one viewer alongside a workspace.
pub struct ViewerSpec {
    /// Short machine name (e.g. `"terminal"`, `"browser"`). Used in resource names.
    pub name: String,
    /// Human-readable label shown in the UI (e.g. `"Terminal"`, `"Browser"`).
    pub display_name: String,
    /// OCI image reference for the viewer container.
    pub image: String,
    /// HTTP path prefix the viewer is reachable at (e.g. `"/projects/viewers/my-ws/terminal"`).
    pub path_prefix: String,
    /// Port the viewer container listens on.
    pub port: u16,
    /// Environment variables injected into the container verbatim.
    pub env_vars: Vec<(String, String)>,
    /// When true, adds a URLRewrite filter to the HTTPRoute that strips `path_prefix`
    /// before forwarding to the backend. Use for viewers that serve at `/` (e.g. noVNC).
    /// When false, the full path is forwarded and the viewer must handle its own prefix.
    pub strip_path_prefix: bool,
    /// When true, sets `securityContext.privileged = true` on the viewer container.
    /// Required for hardware-accelerated viewers that need `/dev/kvm`.
    pub privileged: bool,
    /// Host device paths to expose inside the container (e.g. `["/dev/kvm"]`).
    /// Each entry becomes a `hostPath` volume + volume mount.
    pub host_devices: Vec<String>,
}

fn httproute_api(client: &Client) -> Api<DynamicObject> {
    let ar = ApiResource {
        group: HTTPROUTE_GROUP.into(),
        version: HTTPROUTE_VERSION.into(),
        api_version: format!("{HTTPROUTE_GROUP}/{HTTPROUTE_VERSION}"),
        kind: HTTPROUTE_KIND.into(),
        plural: HTTPROUTE_PLURAL.into(),
    };
    Api::namespaced_with(client.clone(), ROUTE_NAMESPACE, &ar)
}

/// Spawns a Deployment, Service, ReferenceGrant, and HTTPRoute for one viewer.
///
/// Resources are named `viewer-{ws_name}-{spec.name}` and split across two namespaces:
/// - `ws-{ws_name}`: Deployment, Service, ReferenceGrant
/// - `mahakam-system`: HTTPRoute (cross-namespace backendRef via the ReferenceGrant)
///
/// The Service always exposes port 80, regardless of the container port, so the
/// gateway proxy does not need to know the internal port.
pub async fn spawn_viewer(client: &Client, ws_name: &str, spec: ViewerSpec) -> anyhow::Result<()> {
    let ns = format!("ws-{ws_name}");
    let name = format!("viewer-{ws_name}-{}", spec.name);
    let port_i32 = spec.port as i32;

    let mut pod_labels = BTreeMap::new();
    pod_labels.insert(SELECTOR_LABEL.to_string(), name.clone());

    let container_env: Vec<EnvVar> = spec
        .env_vars
        .into_iter()
        .map(|(k, v)| EnvVar {
            name: k,
            value: Some(v),
            ..Default::default()
        })
        .collect();

    // Build hostPath volumes + mounts for any requested host devices.
    let (pod_volumes, container_mounts) = if spec.host_devices.is_empty() {
        (None, None)
    } else {
        let volumes = spec
            .host_devices
            .iter()
            .map(|path| {
                let vol_name = path.trim_start_matches('/').replace('/', "-");
                Volume {
                    name: vol_name,
                    host_path: Some(HostPathVolumeSource {
                        path: path.clone(),
                        type_: None,
                    }),
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();
        let mounts = spec
            .host_devices
            .iter()
            .map(|path| {
                let vol_name = path.trim_start_matches('/').replace('/', "-");
                VolumeMount {
                    name: vol_name,
                    mount_path: path.clone(),
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();
        (Some(volumes), Some(mounts))
    };

    let security_context = if spec.privileged {
        Some(SecurityContext {
            privileged: Some(true),
            ..Default::default()
        })
    } else {
        None
    };

    // ── Deployment ────────────────────────────────────────────────────────────
    let deployment = Deployment {
        metadata: ObjectMeta {
            name: Some(name.clone()),
            namespace: Some(ns.clone()),
            ..Default::default()
        },
        spec: Some(DeploymentSpec {
            replicas: Some(1),
            selector: LabelSelector {
                match_labels: Some(pod_labels.clone()),
                ..Default::default()
            },
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(pod_labels.clone()),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: "viewer".to_string(),
                        image: Some(spec.image),
                        image_pull_policy: Some("IfNotPresent".to_string()),
                        ports: Some(vec![ContainerPort {
                            container_port: port_i32,
                            ..Default::default()
                        }]),
                        env: if container_env.is_empty() {
                            None
                        } else {
                            Some(container_env)
                        },
                        security_context,
                        volume_mounts: container_mounts,
                        ..Default::default()
                    }],
                    volumes: pod_volumes,
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
        ..Default::default()
    };

    Api::<Deployment>::namespaced(client.clone(), &ns)
        .create(&PostParams::default(), &deployment)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create viewer Deployment {name}: {e}"))?;
    info!(ws = %ws_name, viewer = %spec.name, resource = %name, "viewer Deployment created");

    // ── Service ───────────────────────────────────────────────────────────────
    // Always exposes port 80 externally; maps to the container's actual port.
    let service = Service {
        metadata: ObjectMeta {
            name: Some(name.clone()),
            namespace: Some(ns.clone()),
            ..Default::default()
        },
        spec: Some(ServiceSpec {
            selector: Some(pod_labels),
            ports: Some(vec![ServicePort {
                port: 80,
                target_port: Some(IntOrString::Int(port_i32)),
                ..Default::default()
            }]),
            ..Default::default()
        }),
        ..Default::default()
    };

    Api::<Service>::namespaced(client.clone(), &ns)
        .create(&PostParams::default(), &service)
        .await
        .map_err(|e| anyhow::anyhow!("failed to create viewer Service {name}: {e}"))?;
    info!(ws = %ws_name, viewer = %spec.name, resource = %name, "viewer Service created");

    // ── ReferenceGrant ────────────────────────────────────────────────────────
    kubectl_apply(&serde_json::json!({
        "apiVersion": "gateway.networking.k8s.io/v1beta1",
        "kind": "ReferenceGrant",
        "metadata": { "name": format!("allow-mahakam-{}", spec.name), "namespace": ns },
        "spec": {
            "from": [{ "group": "gateway.networking.k8s.io", "kind": "HTTPRoute", "namespace": ROUTE_NAMESPACE }],
            "to":   [{ "group": "", "kind": "Service" }],
        },
    }))
    .await?;
    info!(ws = %ws_name, viewer = %spec.name, "viewer ReferenceGrant applied");

    // ── HTTPRoute ─────────────────────────────────────────────────────────────
    // Labels allow discovery via list_all_ws_viewers.
    // Annotations carry display name and path for the frontend.
    let mut rules_entry = serde_json::json!({
        "matches": [{ "path": { "type": "PathPrefix", "value": spec.path_prefix } }],
        "backendRefs": [{ "name": name, "namespace": ns, "port": 80 }],
    });
    if spec.strip_path_prefix {
        rules_entry["filters"] = serde_json::json!([{
            "type": "URLRewrite",
            "urlRewrite": {
                "path": {
                    "type": "ReplacePrefixMatch",
                    "replacePrefixMatch": "/",
                },
            },
        }]);
    }

    kubectl_apply(&serde_json::json!({
        "apiVersion": "gateway.networking.k8s.io/v1",
        "kind": "HTTPRoute",
        "metadata": {
            "name": name,
            "namespace": ROUTE_NAMESPACE,
            "labels": {
                LABEL_VIEWER: "true",
                LABEL_WS_NAME: ws_name,
            },
            "annotations": {
                ANN_VIEWER_DISPLAY_NAME: spec.display_name,
                ANN_VIEWER_PATH: spec.path_prefix,
            },
        },
        "spec": {
            "parentRefs": [{ "name": GATEWAY_NAME, "namespace": ROUTE_NAMESPACE }],
            "rules": [rules_entry],
        },
    }))
    .await?;
    info!(ws = %ws_name, viewer = %spec.name, route = %name, "viewer HTTPRoute applied");

    Ok(())
}

/// Lists all viewer HTTPRoutes across all workspaces, grouped by workspace name.
///
/// A single list call covers all workspaces; callers merge the result into
/// each `Workspace` returned by `list_ws_applications`.
pub async fn list_all_ws_viewers(client: &Client) -> anyhow::Result<HashMap<String, Vec<Viewer>>> {
    let api = httproute_api(client);
    let lp = ListParams::default().labels(&format!("{LABEL_VIEWER}=true"));
    let routes = api
        .list(&lp)
        .await
        .map_err(|e| anyhow::anyhow!("failed to list viewer HTTPRoutes: {e}"))?;

    let mut map: HashMap<String, Vec<Viewer>> = HashMap::new();

    for route in routes {
        let labels = route.metadata.labels.as_ref().cloned().unwrap_or_default();
        let annotations = route
            .metadata
            .annotations
            .as_ref()
            .cloned()
            .unwrap_or_default();

        let Some(ws_name) = labels.get(LABEL_WS_NAME).cloned() else {
            continue;
        };
        let Some(name) = route.metadata.name.as_deref() else {
            continue;
        };
        let display_name = annotations
            .get(ANN_VIEWER_DISPLAY_NAME)
            .cloned()
            .unwrap_or_else(|| name.to_string());
        let path = annotations
            .get(ANN_VIEWER_PATH)
            .cloned()
            .unwrap_or_default();

        // Derive the short viewer name: strip "viewer-{ws_name}-" prefix from route name.
        let viewer_name = name
            .strip_prefix(&format!("viewer-{ws_name}-"))
            .unwrap_or(name)
            .to_string();

        map.entry(ws_name).or_default().push(Viewer {
            name: viewer_name,
            display_name,
            path,
        });
    }

    Ok(map)
}

/// Removes all viewer HTTPRoutes for `ws_name` from `mahakam-system`.
///
/// The Deployment, Service, and ReferenceGrant live in `ws-{ws_name}` and are
/// cleaned up automatically when that namespace is deleted during ArgoCD cascade.
pub async fn teardown_viewer(client: &Client, ws_name: &str) -> anyhow::Result<()> {
    let api = httproute_api(client);
    let lp =
        ListParams::default().labels(&format!("{LABEL_VIEWER}=true,{LABEL_WS_NAME}={ws_name}"));

    let routes = api
        .list(&lp)
        .await
        .map_err(|e| anyhow::anyhow!("failed to list viewer HTTPRoutes for {ws_name}: {e}"))?;

    for route in routes {
        let route_name = route.metadata.name.as_deref().unwrap_or_default();
        match api.delete(route_name, &DeleteParams::default()).await {
            Ok(_) => info!(ws = %ws_name, route = %route_name, "viewer HTTPRoute deleted"),
            Err(kube::Error::Api(ref e)) if e.code == 404 => {}
            Err(e) => {
                warn!(ws = %ws_name, route = %route_name, error = %e, "failed to delete viewer HTTPRoute")
            }
        }
    }

    Ok(())
}

async fn kubectl_apply(resource: &serde_json::Value) -> anyhow::Result<()> {
    let yaml = serde_yaml::to_string(resource)
        .map_err(|e| anyhow::anyhow!("failed to serialize resource to YAML: {e}"))?;

    let mut child = Command::new("kubectl")
        .args(["apply", "-f", "-"])
        .stdin(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(yaml.as_bytes()).await?;
    }

    let status = child.wait().await?;
    if !status.success() {
        anyhow::bail!("kubectl apply failed");
    }
    Ok(())
}
