use axum::{body::Body, extract::Request, http::Response, middleware::Next};
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
