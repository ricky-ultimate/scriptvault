mod auth;
mod db;
mod error;
mod r2;
mod routes;
mod state;

use axum::{
    routing::{get, post, put},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "scriptvault_server=info,tower_http=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    db::init_tables(&pool).await?;

    let r2 = r2::R2Client::new(
        &std::env::var("R2_ACCOUNT_ID").expect("R2_ACCOUNT_ID must be set"),
        &std::env::var("R2_ACCESS_KEY_ID").expect("R2_ACCESS_KEY_ID must be set"),
        &std::env::var("R2_SECRET_ACCESS_KEY").expect("R2_SECRET_ACCESS_KEY must be set"),
        &std::env::var("R2_BUCKET_NAME").expect("R2_BUCKET_NAME must be set"),
    );

    let state = AppState {
        db: pool,
        r2: Arc::new(r2),
    };

    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/auth/register", post(routes::auth::register))
        .route("/auth/me", get(routes::auth::me))
        .route("/vault", get(routes::vault::get_vault))
        .route("/vault", put(routes::vault::put_vault))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
