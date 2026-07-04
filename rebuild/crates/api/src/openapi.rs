//! OpenAPI 3.1 SSOT (REBUILD-MAP §1: "OpenAPI 3.1 as SSOT ... replaces shared-types/Zod as the
//! cross-boundary type authority"). Phase A only has health + the menu stub annotated; every
//! route added later must gain a `#[utoipa::path(...)]` annotation and a `paths(...)` entry here
//! — `openapi-diff` (REBUILD-MAP §Decision register, CI/CD row) is the gate that keeps this from
//! silently drifting from the real router once there's a generated FE client to diff against.

use utoipa::OpenApi;

use crate::routes::health::HealthStatus;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health::healthz,
        crate::routes::health::livez,
        crate::routes::menu::get_public_menu,
    ),
    components(schemas(HealthStatus)),
    tags(
        (name = "health", description = "Liveness/health probes"),
        (name = "menu", description = "Public storefront menu"),
    )
)]
pub struct ApiDoc;

pub async fn openapi_json() -> axum::Json<utoipa::openapi::OpenApi> {
    axum::Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_document_lists_all_phase_a_routes() {
        let doc = ApiDoc::openapi();
        let paths: Vec<&String> = doc.paths.paths.keys().collect();
        assert!(paths.iter().any(|p| p.as_str() == "/healthz"));
        assert!(paths.iter().any(|p| p.as_str() == "/livez"));
        assert!(
            paths
                .iter()
                .any(|p| p.as_str() == "/api/v1/public/menu/{slug}")
        );
    }
}
