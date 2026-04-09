use std::collections::BTreeMap;

use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::{ObjectMeta, PostParams},
    Api, Client,
};
use tokio::time::{sleep, timeout, Duration};
use tracing::{info, warn};

/// Creates (or silently accepts an existing) namespace for the given environment.
pub async fn create_env_namespace(client: &Client, env_name: &str) -> anyhow::Result<()> {
    let namespaces: Api<Namespace> = Api::all(client.clone());
    let ns_name = format!("env-{env_name}");

    let mut labels = BTreeMap::new();
    labels.insert("mahakam.io/managed".to_string(), "true".to_string());
    labels.insert("mahakam.io/env".to_string(), env_name.to_string());

    let ns = Namespace {
        metadata: ObjectMeta {
            name: Some(ns_name.clone()),
            labels: Some(labels),
            ..Default::default()
        },
        ..Default::default()
    };

    match namespaces.create(&PostParams::default(), &ns).await {
        Ok(_) => {
            info!(namespace = %ns_name, "created namespace");
            Ok(())
        }
        Err(kube::Error::Api(e)) if e.code == 409 => {
            // Namespace already exists — check if it is Terminating.
            let existing = namespaces.get(&ns_name).await?;
            let phase = existing
                .status
                .as_ref()
                .and_then(|s| s.phase.as_deref())
                .unwrap_or("");
            if phase == "Terminating" {
                warn!(namespace = %ns_name, "namespace is Terminating — waiting for deletion");
                wait_for_namespace_gone(&namespaces, &ns_name).await?;
                info!(namespace = %ns_name, "namespace gone, creating fresh");
                namespaces.create(&PostParams::default(), &ns).await?;
                info!(namespace = %ns_name, "created namespace");
            } else {
                info!(namespace = %ns_name, "namespace already exists");
            }
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// Polls until the namespace no longer exists (times out after 120 s).
async fn wait_for_namespace_gone(namespaces: &Api<Namespace>, ns_name: &str) -> anyhow::Result<()> {
    timeout(Duration::from_secs(120), async {
        loop {
            match namespaces.get(ns_name).await {
                Err(kube::Error::Api(e)) if e.code == 404 => return Ok::<(), anyhow::Error>(()),
                Err(e) => return Err(anyhow::anyhow!(e)),
                Ok(_) => {
                    sleep(Duration::from_secs(3)).await;
                }
            }
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("timed out waiting for namespace {ns_name} to be deleted"))?
}

/// Deletes the namespace for the given environment.
pub async fn delete_env_namespace(client: &Client, env_name: &str) -> anyhow::Result<()> {
    let namespaces: Api<Namespace> = Api::all(client.clone());
    let ns_name = format!("env-{env_name}");

    match namespaces
        .delete(&ns_name, &kube::api::DeleteParams::default())
        .await
    {
        Ok(_) => {
            info!(namespace = %ns_name, "deleted namespace");
            Ok(())
        }
        Err(kube::Error::Api(e)) if e.code == 404 => {
            info!(namespace = %ns_name, "namespace not found, skipping delete");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
