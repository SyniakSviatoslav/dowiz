//! `/healthz` (process is up) and `/livez` (process is still making progress — the k8s/Fly
//! liveness-probe convention). Neither touches the database in Phase A: a readiness probe that
//! depends on Postgres belongs on a separate `/readyz` once a real DB-backed route exists, so a
//! transient DB blip doesn't flap the whole process's liveness.

use axum::Json;
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthStatus {
    status: &'static str,
}

#[utoipa::path(
    get,
    path = "/healthz",
    responses((status = 200, description = "process is up", body = HealthStatus)),
    tag = "health"
)]
pub async fn healthz() -> Json<HealthStatus> {
    Json(HealthStatus { status: "ok" })
}

#[utoipa::path(
    get,
    path = "/livez",
    responses((status = 200, description = "process is live", body = HealthStatus)),
    tag = "health"
)]
pub async fn livez() -> Json<HealthStatus> {
    Json(HealthStatus { status: "ok" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn healthz_reports_ok() {
        let Json(body) = healthz().await;
        assert_eq!(body.status, "ok");
    }

    #[tokio::test]
    async fn livez_reports_ok() {
        let Json(body) = livez().await;
        assert_eq!(body.status, "ok");
    }
}
