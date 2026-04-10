use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use tracing::error;

use crate::routes::AppState;

/// A logical project grouping for workspaces.
#[derive(Serialize)]
pub struct Project {
    pub name: String,
    pub workspace_count: u32,
}

/// GET /api/v1/projects — derives the project list from ArgoCD Application annotations.
///
/// Projects are not stored independently; they are derived at query time from the
/// `mahakam.io/ws-project` annotation on each workspace Application.
/// The `"default"` project is always included even when empty.
pub async fn list_projects(State(state): State<AppState>) -> impl IntoResponse {
    let workspaces =
        match crate::k8s::argocd::list_ws_applications(&state.kube_client, &state.argocd_namespace)
            .await
        {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to list workspaces for projects: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }
        };

    let mut counts: HashMap<String, u32> = HashMap::new();
    for ws in &workspaces {
        *counts.entry(ws.project.clone()).or_insert(0) += 1;
    }
    counts.entry("default".to_string()).or_insert(0);

    let mut projects: Vec<Project> = counts
        .into_iter()
        .map(|(name, workspace_count)| Project {
            name,
            workspace_count,
        })
        .collect();
    projects.sort_by(|a, b| a.name.cmp(&b.name));

    (StatusCode::OK, Json(projects)).into_response()
}

/// GET /api/v1/projects/:name/workspaces — workspaces belonging to the given project.
pub async fn list_project_workspaces(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> impl IntoResponse {
    let workspaces =
        match crate::k8s::argocd::list_ws_applications(&state.kube_client, &state.argocd_namespace)
            .await
        {
            Ok(w) => w,
            Err(e) => {
                error!("Failed to list workspaces for project {}: {}", project, e);
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }
        };

    let viewer_map = match crate::k8s::viewer::list_all_ws_viewers(&state.kube_client).await {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to list viewers for project {}: {}", project, e);
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    let result: Vec<_> = workspaces
        .into_iter()
        .filter(|ws| ws.project == project)
        .map(|mut ws| {
            ws.viewers = viewer_map.get(&ws.name).cloned().unwrap_or_default();
            ws
        })
        .collect();

    (StatusCode::OK, Json(result)).into_response()
}
