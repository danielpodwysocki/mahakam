use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use tracing::{error, info, warn};
use uuid::Uuid;

use shared::{repositories::workspace::Workspace, services::workspace::validate_name};

use crate::routes::AppState;

#[derive(Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub repos: Vec<String>,
    /// Viewer types to spawn (e.g. `["terminal", "browser"]`).
    pub viewers: Vec<String>,
    /// Project this workspace belongs to. Defaults to `"default"`.
    pub project: Option<String>,
}

/// GET /api/v1/workspaces — returns all workspaces with their active viewers.
///
/// Viewers are discovered dynamically from HTTPRoute labels in `mahakam-system`;
/// one list call covers all workspaces.
pub async fn list_workspaces(State(state): State<AppState>) -> impl IntoResponse {
    let workspaces =
        match crate::k8s::argocd::list_ws_applications(&state.kube_client, &state.argocd_namespace)
            .await
        {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to list workspaces: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }
        };

    let viewer_map = match crate::k8s::viewer::list_all_ws_viewers(&state.kube_client).await {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to list viewers: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    let workspaces: Vec<Workspace> = workspaces
        .into_iter()
        .map(|mut ws| {
            ws.viewers = viewer_map.get(&ws.name).cloned().unwrap_or_default();
            ws
        })
        .collect();

    (StatusCode::OK, Json(workspaces)).into_response()
}

/// POST /api/v1/workspaces — creates an ArgoCD Application and returns immediately.
///
/// Provisioning (vcluster readiness, kustomization, viewer spawn) runs in the
/// background. The `viewers` field controls which viewer types are spawned.
pub async fn create_workspace(
    State(state): State<AppState>,
    Json(body): Json<CreateWorkspaceRequest>,
) -> impl IntoResponse {
    if let Err(e) = validate_name(&body.name) {
        return (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()).into_response();
    }

    let project = body.project.unwrap_or_else(|| "default".to_string());
    let ws = Workspace {
        id: Uuid::new_v4().to_string(),
        namespace: format!("ws-{}", body.name),
        name: body.name,
        repos: body.repos,
        status: "pending".to_string(),
        created_at: Utc::now().to_rfc3339(),
        viewers: vec![],
        project,
    };

    if let Err(e) = crate::k8s::argocd::create_ws_application(
        &state.kube_client,
        &crate::k8s::argocd::WsApplicationSpec {
            ws_name: &ws.name,
            ws_id: &ws.id,
            ws_repos: &ws.repos,
            ws_created_at: &ws.created_at,
            repo_url: &state.repo_url,
            repo_revision: &state.repo_revision,
            argocd_namespace: &state.argocd_namespace,
            argocd_project: "default",
            vcluster_chart_version: &state.vcluster_chart_version,
            ws_project: &ws.project,
        },
    )
    .await
    {
        error!(ws = %ws.name, error = %e, "failed to create ArgoCD Application");
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    let kube_client = state.kube_client.clone();
    let base_path = state.base_path.clone();
    let viewer_image = state.viewer_image.clone();
    let browser_viewer_image = state.browser_viewer_image.clone();
    let android_viewer_image = state.android_viewer_image.clone();
    let argocd_namespace = state.argocd_namespace.clone();
    let ws_name = ws.name.clone();
    let ws_repos = ws.repos.clone();
    let requested_viewers = body.viewers;

    tokio::spawn(async move {
        info!(ws = %ws_name, "background provisioning started");
        let result: anyhow::Result<()> = async {
            crate::k8s::argocd::wait_for_ws_healthy(&kube_client, &ws_name, &argocd_namespace)
                .await?;

            let kubeconfig =
                crate::k8s::vcluster::wait_for_vcluster_kubeconfig(&kube_client, &ws_name).await?;

            crate::k8s::vcluster::wait_for_vcluster_api_ready(&ws_name, &kubeconfig).await?;

            crate::k8s::kustomize::apply_ws_kustomization(
                &ws_name,
                &ws_repos,
                &base_path,
                &kubeconfig,
            )
            .await?;

            for viewer_type in &requested_viewers {
                let spec = viewer_spec_for(
                    &ws_name,
                    viewer_type,
                    &viewer_image,
                    &browser_viewer_image,
                    &android_viewer_image,
                );
                match spec {
                    Some(s) => {
                        crate::k8s::viewer::spawn_viewer(&kube_client, &ws_name, s).await?;
                    }
                    None => {
                        warn!(ws = %ws_name, viewer = %viewer_type, "unknown viewer type, skipping")
                    }
                }
            }

            Ok(())
        }
        .await;

        match result {
            Ok(()) => {
                info!(ws = %ws_name, "provisioning complete");
            }
            Err(ref e) => {
                error!(ws = %ws_name, error = %e, "provisioning failed");
                if let Err(patch_err) = crate::k8s::argocd::update_ws_application_status(
                    &kube_client,
                    &ws_name,
                    &argocd_namespace,
                    "failed",
                )
                .await
                {
                    error!(ws = %ws_name, error = %patch_err, "failed to update status annotation");
                }
            }
        }
    });

    (StatusCode::CREATED, Json(ws)).into_response()
}

/// GET /api/v1/workspaces/:name — returns a single workspace by name.
pub async fn get_workspace(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let workspaces =
        match crate::k8s::argocd::list_ws_applications(&state.kube_client, &state.argocd_namespace)
            .await
        {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to list workspaces: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }
        };

    let viewer_map = match crate::k8s::viewer::list_all_ws_viewers(&state.kube_client).await {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to list viewers: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    match workspaces.into_iter().find(|ws| ws.name == name) {
        Some(mut ws) => {
            ws.viewers = viewer_map.get(&ws.name).cloned().unwrap_or_default();
            (StatusCode::OK, Json(ws)).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// GET /api/v1/workspaces/:name/metrics — pod count and resource usage in the workspace namespace.
pub async fn get_workspace_metrics(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match crate::k8s::metrics::get_ws_metrics(&state.kube_client, &name).await {
        Ok(metrics) => (StatusCode::OK, Json(metrics)).into_response(),
        Err(e) => {
            error!("Failed to get metrics for {}: {}", name, e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

/// DELETE /api/v1/workspaces/:name
pub async fn delete_workspace(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = crate::k8s::viewer::teardown_viewer(&state.kube_client, &name).await {
        error!("Failed to teardown viewers for {}: {}", name, e);
    }

    match crate::k8s::argocd::delete_ws_application(
        &state.kube_client,
        &name,
        &state.argocd_namespace,
    )
    .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            error!("Failed to delete ArgoCD Application for {}: {}", name, e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

/// Maps a viewer type name to its `ViewerSpec`. Returns `None` for unknown types.
fn viewer_spec_for(
    ws_name: &str,
    viewer_type: &str,
    terminal_image: &str,
    browser_image: &str,
    android_image: &str,
) -> Option<crate::k8s::viewer::ViewerSpec> {
    match viewer_type {
        "terminal" => Some(crate::k8s::viewer::ViewerSpec {
            name: "terminal".to_string(),
            display_name: "Terminal".to_string(),
            image: terminal_image.to_string(),
            path_prefix: format!("/projects/viewers/{ws_name}/terminal"),
            port: 7681,
            env_vars: vec![("WS_NAME".to_string(), ws_name.to_string())],
            strip_path_prefix: false,
            privileged: false,
            host_devices: vec![],
        }),
        "browser" => Some(crate::k8s::viewer::ViewerSpec {
            name: "browser".to_string(),
            display_name: "Browser".to_string(),
            image: browser_image.to_string(),
            path_prefix: format!("/projects/viewers/{ws_name}/browser"),
            port: 6080,
            // WS_NAME is injected so nginx inside the container handles the
            // prefix natively — no gateway-level path rewrite needed.
            env_vars: vec![("WS_NAME".to_string(), ws_name.to_string())],
            strip_path_prefix: false,
            privileged: false,
            host_devices: vec![],
        }),
        "android" => Some(crate::k8s::viewer::ViewerSpec {
            name: "android".to_string(),
            display_name: "Android Emulator".to_string(),
            image: android_image.to_string(),
            path_prefix: format!("/projects/viewers/{ws_name}/android"),
            port: 6080,
            env_vars: vec![("WS_NAME".to_string(), ws_name.to_string())],
            strip_path_prefix: false,
            // KVM access is required for hardware-accelerated virtualisation;
            // without it the emulator falls back to swiftshader_indirect.
            privileged: true,
            host_devices: vec!["/dev/kvm".to_string()],
        }),
        _ => None,
    }
}
