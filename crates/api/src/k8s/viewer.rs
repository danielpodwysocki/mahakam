use std::collections::BTreeMap;
use std::process::Stdio;

use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{
    Container, ContainerPort, EnvVar, PodSpec, PodTemplateSpec, Service, ServicePort, ServiceSpec,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::{
    api::{Api, ObjectMeta, PostParams},
    Client,
};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{info, warn};

const SELECTOR_LABEL: &str = "mahakam.io/viewer";
const ROUTE_NAMESPACE: &str = "mahakam-system";
const GATEWAY_NAME: &str = "mahakam";

/// All the information needed to spawn a viewer alongside an environment.
///
/// Callers construct this for a specific viewer type (e.g. ttyd) and pass it
/// to [`spawn_viewer`]. Adding a new viewer type means constructing a different
/// `ViewerSpec` — no changes needed in the spawn/teardown machinery.
pub struct ViewerSpec {
    /// OCI image reference for the viewer container.
    pub image: String,
    /// HTTP path prefix the viewer is reachable at (e.g. `"/projects/viewers/my-env"`).
    pub path_prefix: String,
    /// Port the viewer container listens on.
    pub port: u16,
    /// Environment variables injected into the container verbatim.
    pub env_vars: Vec<(String, String)>,
}

/// Spawns a viewer Deployment, Service, ReferenceGrant, and HTTPRoute for `env_name`.
///
/// Resources are split across two namespaces:
/// - `env-{env_name}`: Deployment, Service, ReferenceGrant
/// - `mahakam-system`: HTTPRoute (cross-namespace backendRef via the ReferenceGrant)
pub async fn spawn_viewer(client: &Client, env_name: &str, spec: ViewerSpec) -> anyhow::Result<()> {
    let ns = format!("env-{env_name}");
    let name = format!("viewer-{env_name}");
    let port_i32 = spec.port as i32;

    let mut pod_labels = BTreeMap::new();
    pod_labels.insert(SELECTOR_LABEL.to_string(), env_name.to_string());

    let container_env: Vec<EnvVar> = spec
        .env_vars
        .into_iter()
        .map(|(k, v)| EnvVar {
            name: k,
            value: Some(v),
            ..Default::default()
        })
        .collect();

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
                        ..Default::default()
                    }],
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
    info!(env = %env_name, resource = %name, "viewer deployment created");

    // ── Service ───────────────────────────────────────────────────────────────
    let service = Service {
        metadata: ObjectMeta {
            name: Some(name.clone()),
            namespace: Some(ns.clone()),
            ..Default::default()
        },
        spec: Some(ServiceSpec {
            selector: Some(pod_labels),
            ports: Some(vec![ServicePort {
                port: port_i32,
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
    info!(env = %env_name, resource = %name, "viewer service created");

    // ── ReferenceGrant ────────────────────────────────────────────────────────
    // Allows the HTTPRoute in mahakam-system to reference the Service in env-{name}.
    kubectl_apply(&format!(
        "apiVersion: gateway.networking.k8s.io/v1beta1\n\
kind: ReferenceGrant\n\
metadata:\n\
  name: allow-mahakam-gateway\n\
  namespace: {ns}\n\
spec:\n\
  from:\n\
    - group: gateway.networking.k8s.io\n\
      kind: HTTPRoute\n\
      namespace: {route_ns}\n\
  to:\n\
    - group: \"\"\n\
      kind: Service\n",
        ns = ns,
        route_ns = ROUTE_NAMESPACE,
    ))
    .await?;
    info!(env = %env_name, "viewer ReferenceGrant applied");

    // ── HTTPRoute ─────────────────────────────────────────────────────────────
    // Lives in mahakam-system, routes /projects/viewers/{name}/* to the viewer Service.
    kubectl_apply(&format!(
        "apiVersion: gateway.networking.k8s.io/v1\n\
kind: HTTPRoute\n\
metadata:\n\
  name: {name}\n\
  namespace: {route_ns}\n\
spec:\n\
  parentRefs:\n\
    - name: {gw}\n\
      namespace: {route_ns}\n\
  rules:\n\
    - matches:\n\
        - path:\n\
            type: PathPrefix\n\
            value: {path}\n\
      backendRefs:\n\
        - name: {name}\n\
          namespace: {ns}\n\
          port: {port}\n",
        name = name,
        route_ns = ROUTE_NAMESPACE,
        gw = GATEWAY_NAME,
        path = spec.path_prefix,
        ns = ns,
        port = port_i32,
    ))
    .await?;
    info!(env = %env_name, route = %name, "viewer HTTPRoute applied");

    Ok(())
}

/// Removes the viewer HTTPRoute for `env_name`.
///
/// The Deployment, Service, and ReferenceGrant live in `env-{env_name}` and are cleaned
/// up automatically when that namespace is deleted during environment teardown. Only the
/// HTTPRoute in `mahakam-system` needs explicit removal.
pub async fn teardown_viewer(env_name: &str) -> anyhow::Result<()> {
    let route_name = format!("viewer-{env_name}");

    let status = Command::new("kubectl")
        .args([
            "delete",
            "httproute",
            &route_name,
            "-n",
            ROUTE_NAMESPACE,
            "--ignore-not-found",
        ])
        .status()
        .await?;

    if status.success() {
        info!(env = %env_name, route = %route_name, "viewer HTTPRoute removed");
    } else {
        warn!(env = %env_name, "kubectl delete httproute returned non-zero (may not exist)");
    }

    Ok(())
}

async fn kubectl_apply(yaml: &str) -> anyhow::Result<()> {
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
