use axum::http::StatusCode;

/// Returns 200 OK. Used for liveness probes.
pub async fn health_check() -> StatusCode {
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_check_returns_ok() {
        assert_eq!(health_check().await, StatusCode::OK);
    }
}
