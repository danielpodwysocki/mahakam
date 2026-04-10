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

use shared::{repositories::environment::Environment, services::environment::validate_name};

use crate::routes::AppState;

#[derive(Deserialize)]
pub struct CreateEnvironmentRequest {
    pub name: String,
    pub repos: Vec<String>,
    /// Viewer types to spawn (e.g. `["terminal", "browser"]`).
    pub viewers: Vec<String>,
}

/// GET /api/v1/environments — returns all environments with their active viewers.
///
/// Viewers are discovered dynamically from HTTPRoute labels in `mahakam-system`;
/// one list call covers all environments.
pub async fn list_environments(State(state): State<AppState>) -> impl IntoResponse {
    let envs = match crate::k8s::argocd::list_env_applications(
        &state.kube_client,
        &state.argocd_namespace,
    )
    .await
    {
        Ok(e) => e,
        Err(e) => {
            error!("Failed to list environments: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    let viewer_map = match crate::k8s::viewer::list_all_env_viewers(&state.kube_client).await {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to list viewers: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    let envs: Vec<Environment> = envs
        .into_iter()
        .map(|mut env| {
            env.viewers = viewer_map.get(&env.name).cloned().unwrap_or_default();
            env
        })
        .collect();

    (StatusCode::OK, Json(envs)).into_response()
}

/// POST /api/v1/environments — creates an ArgoCD Application and returns immediately.
///
/// Provisioning (vcluster readiness, kustomization, viewer spawn) runs in the
/// background. The `viewers` field controls which viewer types are spawned.
pub async fn create_environment(
    State(state): State<AppState>,
    Json(body): Json<CreateEnvironmentRequest>,
) -> impl IntoResponse {
    if let Err(e) = validate_name(&body.name) {
        return (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()).into_response();
    }

    let env = Environment {
        id: Uuid::new_v4().to_string(),
        namespace: format!("env-{}", body.name),
        name: body.name,
        repos: body.repos,
        status: "pending".to_string(),
        created_at: Utc::now().to_rfc3339(),
        viewers: vec![],
    };

    if let Err(e) = crate::k8s::argocd::create_env_application(
        &state.kube_client,
        &crate::k8s::argocd::EnvApplicationSpec {
            env_name: &env.name,
            env_id: &env.id,
            env_repos: &env.repos,
            env_created_at: &env.created_at,
            repo_url: &state.repo_url,
            repo_revision: &state.repo_revision,
            argocd_namespace: &state.argocd_namespace,
            argocd_project: "default",
            vcluster_chart_version: &state.vcluster_chart_version,
        },
    )
    .await
    {
        error!(env = %env.name, error = %e, "failed to create ArgoCD Application");
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    let kube_client = state.kube_client.clone();
    let base_path = state.base_path.clone();
    let viewer_image = state.viewer_image.clone();
    let browser_viewer_image = state.browser_viewer_image.clone();
    let argocd_namespace = state.argocd_namespace.clone();
    let env_name = env.name.clone();
    let env_repos = env.repos.clone();
    let requested_viewers = body.viewers;

    tokio::spawn(async move {
        info!(env = %env_name, "background provisioning started");
        let result: anyhow::Result<()> = async {
            crate::k8s::argocd::wait_for_env_healthy(&kube_client, &env_name, &argocd_namespace)
                .await?;

            let kubeconfig =
                crate::k8s::vcluster::wait_for_vcluster_kubeconfig(&kube_client, &env_name)
                    .await?;

            crate::k8s::vcluster::wait_for_vcluster_api_ready(&env_name, &kubeconfig).await?;

            crate::k8s::kustomize::apply_env_kustomization(
                &env_name,
                &env_repos,
                &base_path,
                &kubeconfig,
            )
            .await?;

            for viewer_type in &requested_viewers {
                let spec = viewer_spec_for(&env_name, viewer_type, &viewer_image, &browser_viewer_image);
                match spec {
                    Some(s) => {
                        crate::k8s::viewer::spawn_viewer(&kube_client, &env_name, s).await?;
                    }
                    None => warn!(env = %env_name, viewer = %viewer_type, "unknown viewer type, skipping"),
                }
            }

            Ok(())
        }
        .await;

        match result {
            Ok(()) => {
                info!(env = %env_name, "provisioning complete");
            }
            Err(ref e) => {
                error!(env = %env_name, error = %e, "provisioning failed");
                if let Err(patch_err) = crate::k8s::argocd::update_env_application_status(
                    &kube_client,
                    &env_name,
                    &argocd_namespace,
                    "failed",
                )
                .await
                {
                    error!(env = %env_name, error = %patch_err, "failed to update status annotation");
                }
            }
        }
    });

    (StatusCode::CREATED, Json(env)).into_response()
}

/// DELETE /api/v1/environments/:name
pub async fn delete_environment(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = crate::k8s::viewer::teardown_viewer(&state.kube_client, &name).await {
        error!("Failed to teardown viewers for {}: {}", name, e);
    }

    match crate::k8s::argocd::delete_env_application(
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
    env_name: &str,
    viewer_type: &str,
    terminal_image: &str,
    browser_image: &str,
) -> Option<crate::k8s::viewer::ViewerSpec> {
    match viewer_type {
        "terminal" => Some(crate::k8s::viewer::ViewerSpec {
            name: "terminal".to_string(),
            display_name: "Terminal".to_string(),
            image: terminal_image.to_string(),
            path_prefix: format!("/projects/viewers/{env_name}/terminal"),
            port: 7681,
            env_vars: vec![("ENV_NAME".to_string(), env_name.to_string())],
            strip_path_prefix: false,
        }),
        "browser" => Some(crate::k8s::viewer::ViewerSpec {
            name: "browser".to_string(),
            display_name: "Browser".to_string(),
            image: browser_image.to_string(),
            path_prefix: format!("/projects/viewers/{env_name}/browser"),
            port: 6080,
            // ENV_NAME is injected so nginx inside the container handles the
            // prefix natively — no gateway-level path rewrite needed.
            env_vars: vec![("ENV_NAME".to_string(), env_name.to_string())],
            strip_path_prefix: false,
        }),
        _ => None,
    }
}
