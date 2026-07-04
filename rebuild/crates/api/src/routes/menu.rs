//! `GET /api/v1/public/menu/:slug` — stub only. The real query (`read_public_menu` DEFINER-fn
//! call via `with_tenant`, REBUILD-MAP inventory/12 §9) lands after the OpenAPI contract lane
//! (REBUILD-MAP §4/Phase A). Returns 501 with a typed ADR-0010 envelope, not a silent 404 or an
//! ad-hoc `{error}` shape, so a client written against the eventual contract already gets the
//! right error *shape* today.

use axum::extract::{Extension, Path};
use tower_http::request_id::RequestId;

use crate::error::ApiError;
use domain::ErrorCode;

#[utoipa::path(
    get,
    path = "/api/v1/public/menu/{slug}",
    params(("slug" = String, Path, description = "Location slug")),
    responses(
        (status = 501, description = "Not implemented — query lands after the OpenAPI contract lane")
    ),
    tag = "menu"
)]
pub async fn get_public_menu(
    Path(slug): Path<String>,
    Extension(request_id): Extension<RequestId>,
) -> ApiError {
    // TODO(contract-lane, REBUILD-MAP Phase A/B S1): wire read_public_menu(slug) via
    // with_tenant once the OpenAPI contract is extracted from the 236-route census. Do NOT
    // re-implement menu assembly here — the DEFINER function stays the single source, called
    // via query_scalar!, with byte-parity against the Node output as the cutover oracle
    // (REBUILD-MAP inventory/12 §9).
    let _slug = slug;
    ApiError::new(
        ErrorCode::NotImplemented,
        "menu query lands after the OpenAPI contract lane",
        correlation_id_string(&request_id),
    )
}

fn correlation_id_string(request_id: &RequestId) -> String {
    request_id
        .header_value()
        .to_str()
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn returns_501_with_not_implemented_code() {
        let request_id = RequestId::new(HeaderValue::from_static("test-correlation-id"));
        let response = get_public_menu(Path("demo".to_string()), Extension(request_id))
            .await
            .into_response();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_IMPLEMENTED);
    }

    #[test]
    fn correlation_id_falls_back_when_header_is_not_utf8() {
        let request_id = RequestId::new(HeaderValue::from_bytes(&[0xff, 0xfe]).unwrap());
        assert_eq!(correlation_id_string(&request_id), "unknown");
    }
}
