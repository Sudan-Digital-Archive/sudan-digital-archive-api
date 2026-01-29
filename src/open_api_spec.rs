use crate::models::request::{
    AccessionPagination, AccessionPaginationWithPrivate, AuthorizeRequest, CreateAccessionRequest,
    CreateAccessionRequestRaw, CreateSubjectRequest, DeleteSubjectRequest, LoginRequest,
    SubjectPagination, UpdateAccessionRequest,
};
use crate::models::response::{
    CreateApiKeyResponse, GetOneAccessionResponse, ListAccessionsResponse, ListSubjectsArResponse,
    ListSubjectsEnResponse, SubjectResponse,
};
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "jwt_cookie_auth",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("jwt"))),
        );
        components.add_security_scheme(
            "api_key_auth",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Api-Key"))),
        );
    }
}

/// OpenAPI specification for the Sudan Digital Archive API
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health::healthcheck,
        crate::routes::accessions::create_accession_crawl,
        crate::routes::accessions::create_accession_raw,
        crate::routes::accessions::get_one_accession,
        crate::routes::accessions::get_one_private_accession,
        crate::routes::accessions::list_accessions,
        crate::routes::accessions::list_accessions_private,
        crate::routes::accessions::delete_accession,
        crate::routes::accessions::update_accession,
        crate::routes::auth::login,
        crate::routes::auth::authorize,
        crate::routes::auth::verify,
        crate::routes::auth::create_api_key,
        crate::routes::subjects::create_subject,
        crate::routes::subjects::list_subjects,
        crate::routes::subjects::delete_subject
    ),
    components(
        schemas(
            AccessionPagination,
            AccessionPaginationWithPrivate,
            CreateAccessionRequest,
            CreateAccessionRequestRaw,
            UpdateAccessionRequest,
            GetOneAccessionResponse,
            ListAccessionsResponse,
            LoginRequest,
            AuthorizeRequest,
            CreateApiKeyResponse,
            CreateSubjectRequest,
            DeleteSubjectRequest,
            SubjectPagination,
            SubjectResponse,
            ListSubjectsEnResponse,
            ListSubjectsArResponse
        )
    ),
    tags(
        (name = "Healthcheck", description = "Health check endpoints"),
        (name = "Accessions", description = "Accession management endpoints"),
        (name = "Auth", description = "User authentication endpoints"),
        (name = "Subjects", description = "Subject management endpoints")
    ),
    modifiers(&SecurityAddon),
    servers(
        // Deployed on Digital Ocean spaces which has a HTTP request config that slaps on this sda-api prefix
        (url = "/sda-api", description = "Production deployment with prefix"),
        (url = "/", description = "Local development without prefix")
    )
)]
pub struct ApiDoc;
