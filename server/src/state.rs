use sqlx::PgPool;
use std::sync::Arc;
use crate::r2::R2Client;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub r2: Arc<R2Client>,
}
