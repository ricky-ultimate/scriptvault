use axum::{
    body::Body,
    extract::Request,
    http::{Response, StatusCode},
    middleware::Next,
};
use governor::{
    DefaultDirectRateLimiter, Quota, RateLimiter,
    clock::DefaultClock,
    state::keyed::DefaultKeyedStateStore,
};
use std::{
    net::IpAddr,
    num::NonZeroU32,
    sync::{Arc, OnceLock},
};
use uuid::Uuid;

pub async fn request_id(mut req: Request, next: Next) -> Response<Body> {
    let request_id = Uuid::new_v4().to_string();
    req.extensions_mut().insert(RequestId(request_id.clone()));
    let mut response = next.run(req).await;
    response
        .headers_mut()
        .insert("x-request-id", request_id.parse().unwrap());
    response
}

#[derive(Clone)]
pub struct RequestId(pub String);

type IpRateLimiter = Arc<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>;

fn global_limiter() -> &'static IpRateLimiter {
    static LIMITER: OnceLock<IpRateLimiter> = OnceLock::new();
    LIMITER.get_or_init(|| {
        Arc::new(RateLimiter::keyed(
            Quota::per_minute(NonZeroU32::new(120).unwrap()),
        ))
    })
}

pub async fn rate_limit(req: Request, next: Next) -> Response<Body> {
    let ip = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));

    if global_limiter().check_key(&ip).is_err() {
        return Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("content-type", "application/json")
            .body(Body::from(r#"{"error":"rate limit exceeded"}"#))
            .unwrap();
    }

    next.run(req).await
}
