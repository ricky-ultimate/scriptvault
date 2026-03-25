mod auth;
mod db;
mod error;
mod middleware;
mod r2;
mod routes;
mod state;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "scriptvault_server=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting ScriptVault server");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let max_connections: u32 = std::env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);

    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(&database_url)
        .await?;
    tracing::info!("Connected to database");

    db::run_migrations(&pool).await?;
    tracing::info!("Database migrations applied");

    let r2 = r2::R2Client::new(
        &std::env::var("R2_ACCOUNT_ID").expect("R2_ACCOUNT_ID must be set"),
        &std::env::var("R2_ACCESS_KEY_ID").expect("R2_ACCESS_KEY_ID must be set"),
        &std::env::var("R2_SECRET_ACCESS_KEY").expect("R2_SECRET_ACCESS_KEY must be set"),
        &std::env::var("R2_BUCKET_NAME").expect("R2_BUCKET_NAME must be set"),
    );
    tracing::info!("R2 client initialized");

    let state = AppState {
        db: pool,
        r2: Arc::new(r2),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/auth/register", post(routes::auth::register))
        .route("/auth/me", get(routes::auth::me))
        .route("/scripts", get(routes::scripts::list_scripts))
        .route("/scripts/:id", get(routes::scripts::get_script))
        .route("/scripts/:id", put(routes::scripts::put_script))
        .route("/scripts/:id", delete(routes::scripts::delete_script))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(axum::middleware::from_fn(middleware::request_id))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on http://{}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, shutting down");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, shutting down");
        }
    }
}
