use axum::{
    body::Body,
    extract::Request,
    http::{Response, StatusCode},
    middleware::Next,
};
use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DefaultKeyedStateStore};
use std::{
    net::IpAddr,
    num::NonZeroU32,
    sync::{Arc, OnceLock},
};
use uuid::Uuid;

#[derive(Clone)]
pub struct RequestId(pub String);

type IpRateLimiter = Arc<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>;

fn global_limiter() -> &'static IpRateLimiter {
    static LIMITER: OnceLock<IpRateLimiter> = OnceLock::new();
    LIMITER.get_or_init(|| {
        Arc::new(RateLimiter::keyed(Quota::per_minute(
            NonZeroU32::new(120).unwrap(),
        )))
    })
}

pub async fn request_id(mut req: Request, next: Next) -> Response<Body> {
    let id = Uuid::new_v4().to_string();
    req.extensions_mut().insert(RequestId(id.clone()));

    let span = tracing::info_span!("request", request_id = %id);
    let _guard = span.enter();

    let mut response = next.run(req).await;
    response
        .headers_mut()
        .insert("x-request-id", id.parse().unwrap());
    response
}

pub async fn rate_limit(req: Request, next: Next) -> Response<Body> {
    let request_id = req
        .extensions()
        .get::<RequestId>()
        .map(|r| r.0.clone())
        .unwrap_or_default();

    let ip = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));

    if global_limiter().check_key(&ip).is_err() {
        tracing::warn!(request_id = %request_id, ip = %ip, "rate limit exceeded");
        return Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("content-type", "application/json")
            .header("x-request-id", request_id)
            .body(Body::from(r#"{"error":"rate limit exceeded"}"#))
            .unwrap();
    }

    next.run(req).await
}
