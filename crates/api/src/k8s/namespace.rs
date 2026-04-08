use std::collections::BTreeMap;

use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::{ObjectMeta, PostParams},
    Api, Client,
};
use tracing::info;

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
            // AlreadyExists — idempotent
            info!(namespace = %ns_name, "namespace already exists");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
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
