use axum::{
    extract::Path,
    http::{StatusCode, HeaderMap, header},
    response::IntoResponse,
    routing::get,
    Router,
};
use axum::body::Body;
use futures::stream::unfold;

pub struct Faucet {}
impl Faucet {
    const CHUNK_SIZE: usize = 64 * 1024;

    async fn handler(Path(size): Path<usize>) -> impl IntoResponse {
        let stream = unfold(0usize, move |sent| async move {
            if sent >= size {
                return None;
            }
            let chunk_size = std::cmp::min(Self::CHUNK_SIZE, size - sent);
            let bytes: Vec<u8> = (0..chunk_size).map(|i| i as u8).collect();
            Some((Ok::<_, std::convert::Infallible>(bytes), sent + chunk_size))
        });

        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/octet-stream".parse().unwrap());
        (StatusCode::OK, headers, Body::from_stream(stream))
    }

    pub fn routes() -> Router {
        Router::new().route("/faucet/{size}", get(Self::handler))
    }
}
