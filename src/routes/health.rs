/// Healthcheck route for sassy uptime check reporting
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Application is healthy", body = String)
    )
)]
pub async fn healthcheck() -> String {
    "Healthy af".to_string()
}
