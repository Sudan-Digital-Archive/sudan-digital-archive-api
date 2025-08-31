use crate::models::request::{
    AccessionPagination, AccessionPaginationWithPrivate, AuthorizeRequest, CreateAccessionRequest,
    CreateSubjectRequest, DeleteSubjectRequest, LoginRequest, SubjectPagination,
    UpdateAccessionRequest,
};
use crate::models::response::{
    GetOneAccessionResponse, ListAccessionsResponse, ListSubjectsArResponse,
    ListSubjectsEnResponse, SubjectResponse, GetOneAccessionResponseSchema
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
        )
    }
}

/// OpenAPI specification for the Sudan Digital Archive API
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health::healthcheck,
        crate::routes::accessions::create_accession,
        crate::routes::accessions::get_one_accession,
        crate::routes::accessions::get_one_private_accession,
        crate::routes::accessions::list_accessions,
        crate::routes::accessions::list_accessions_private,
        crate::routes::accessions::delete_accession,
        crate::routes::accessions::update_accession,
        crate::routes::auth::login,
        crate::routes::auth::authorize,
        crate::routes::auth::verify,
        crate::routes::subjects::create_subject,
        crate::routes::subjects::list_subjects,
        crate::routes::subjects::delete_subject
    ),
    components(
        schemas(
            AccessionPagination,
            AccessionPaginationWithPrivate,
            CreateAccessionRequest,
            UpdateAccessionRequest,
            GetOneAccessionResponseSchema,
            ListAccessionsResponse,
            LoginRequest,
            AuthorizeRequest,
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
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;
