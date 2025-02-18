use axum::response::Response;
use std::sync::Arc;
use crate::models::request::{CreateAccessionRequest, CreateSubjectRequest};
use crate::repos::subjects_repo::SubjectsRepo;

#[derive(Clone)]
pub struct SubjectsService {
    pub subjects_repo: Arc<dyn SubjectsRepo>,
}

impl SubjectsService {
    pub async fn create_one(self, payload: CreateSubjectRequest) -> Response{
        todo!()
    }

    pub async fn list(self, query: String) -> Response{
        todo!()
    }
}
