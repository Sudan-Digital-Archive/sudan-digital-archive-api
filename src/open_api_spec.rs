use utoipa::OpenApi;

/// OpenAPI specification for the Sudan Digital Archive API
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health::healthcheck
    ),
    tags(
        (name = "Healthcheck", description = "Health check endpoints"),
        (name = "SDA Api", description = "Sudan Digital Archive API")
    )
)]
pub struct ApiDoc;
