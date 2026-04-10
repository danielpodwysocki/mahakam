use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, ListParams},
    Client,
};
use serde::{Deserialize, Serialize};

/// Resource usage summary for a workspace namespace on the host cluster.
#[derive(Debug, Serialize, Deserialize)]
pub struct WsMetrics {
    pub pod_count: u32,
    pub cpu_requests_millicores: u64,
    pub cpu_limits_millicores: u64,
    pub memory_requests_mi: u64,
    pub memory_limits_mi: u64,
}

/// Queries pods in `ws-{ws_name}` on the host cluster and sums container resource requests/limits.
pub async fn get_ws_metrics(client: &Client, ws_name: &str) -> anyhow::Result<WsMetrics> {
    let ns = format!("ws-{ws_name}");
    let pods: Api<Pod> = Api::namespaced(client.clone(), &ns);
    let pod_list = pods
        .list(&ListParams::default())
        .await
        .map_err(|e| anyhow::anyhow!("failed to list pods in {ns}: {e}"))?;

    let pod_count = pod_list.items.len() as u32;
    let mut cpu_req = 0u64;
    let mut cpu_lim = 0u64;
    let mut mem_req = 0u64;
    let mut mem_lim = 0u64;

    for pod in &pod_list.items {
        if let Some(spec) = &pod.spec {
            for container in &spec.containers {
                if let Some(resources) = &container.resources {
                    if let Some(requests) = &resources.requests {
                        if let Some(cpu) = requests.get("cpu") {
                            cpu_req += parse_millicores(&cpu.0);
                        }
                        if let Some(mem) = requests.get("memory") {
                            mem_req += parse_memory_mi(&mem.0);
                        }
                    }
                    if let Some(limits) = &resources.limits {
                        if let Some(cpu) = limits.get("cpu") {
                            cpu_lim += parse_millicores(&cpu.0);
                        }
                        if let Some(mem) = limits.get("memory") {
                            mem_lim += parse_memory_mi(&mem.0);
                        }
                    }
                }
            }
        }
    }

    Ok(WsMetrics {
        pod_count,
        cpu_requests_millicores: cpu_req,
        cpu_limits_millicores: cpu_lim,
        memory_requests_mi: mem_req,
        memory_limits_mi: mem_lim,
    })
}

/// Parses a Kubernetes CPU quantity string into millicores.
///
/// Handles the two common formats: `"100m"` (millicores) and `"1"` (whole cores).
fn parse_millicores(s: &str) -> u64 {
    if let Some(m) = s.strip_suffix('m') {
        m.parse().unwrap_or(0)
    } else {
        s.parse::<u64>().unwrap_or(0) * 1000
    }
}

/// Parses a Kubernetes memory quantity string into mebibytes.
///
/// Handles `Gi`, `Mi`, `Ki`, and raw byte values.
fn parse_memory_mi(s: &str) -> u64 {
    if let Some(v) = s.strip_suffix("Gi") {
        v.parse::<u64>().unwrap_or(0) * 1024
    } else if let Some(v) = s.strip_suffix("Mi") {
        v.parse().unwrap_or(0)
    } else if let Some(v) = s.strip_suffix("Ki") {
        v.parse::<u64>().unwrap_or(0) / 1024
    } else {
        s.parse::<u64>().unwrap_or(0) / (1024 * 1024)
    }
}
