use rust_embed::Embed;
use axum::{
    extract::Path,
    http::{StatusCode, HeaderMap, header},
    response::IntoResponse,
    routing::get,
    Router,
};
use mime_guess::from_path;
use time::macros::datetime;

#[derive(Embed)]
#[folder = "data/"]
pub struct Data;

#[derive(Embed)]
#[folder = "static/"]
pub struct Static;

impl Data {
    pub fn get_str(file_path: &str) -> String {
        let buf = Data::get(file_path).unwrap();
        let s = match std::str::from_utf8(buf.data.as_ref()) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        return s.to_string();
    }
}

pub struct StaticFiles {}
impl StaticFiles {
    pub fn find(path: &str) -> Option<String> {
        Static::get(path).map(|_| format!("/static/{}", path))
    }

    async fn get_reply_for(Path(path): Path<String>) -> impl IntoResponse {
        let f = Static::get(&path);
        if let Some(file) = f {
            let maybe_mime = from_path(&path).first();

            let mut headers = HeaderMap::new();
            if let Some(mime) = maybe_mime {
                let content_type = mime.as_ref();
                headers.insert(header::CONTENT_TYPE, content_type.parse().unwrap());
            }
            (StatusCode::OK, headers, file.data.to_vec())
        } else {
            (StatusCode::NOT_FOUND, HeaderMap::new(), Vec::new())
        }
    }

    pub fn routes() -> Router {
        Router::new().route("/static/{*path}", get(Self::get_reply_for))
    }
}

/// A weblog entry
pub struct Entry {
    /// Slug
    pub slug: &'static str,
    /// Title
    pub title: &'static str,
    /// Publish datetime
    pub published_on: time::OffsetDateTime,
    /// Path in the data directory
    pub at: &'static str,
}

impl Entry {
    pub fn find(slug: &str) -> Option<String> {
        WEBLOG_ENTRIES
            .iter()
            .find(|entry| entry.slug == slug)
            .map(|_| format!("/weblog/{}", slug))
    }
}

pub static WEBLOG_ENTRIES: [Entry; 5] = [
    Entry {
        slug: "ls2j",
        title: "LS2J",
        published_on: datetime!(2023-12-14 4:45 pm -8),
        at: "weblog/ls2j.md",
    },
    Entry {
        slug: "thread-equivalence-checker",
        title: "Thread Equivalence Checking - Part 1",
        published_on: datetime!(2024-07-08 8:20 pm -7),
        at: "weblog/thread-equivalence-checker.md",
    },
    Entry {
        slug: "thread-equivalence-checker-2",
        title: "Thread Equivalence Checking - Part 2",
        published_on: datetime!(2024-08-19 4:57 pm +2),
        at: "weblog/thread-equivalence-checker-2.md",
    },
    Entry {
        slug: "thread-equivalence-checker-3",
        title: "Thread Equivalence Checking - Part 3",
        published_on: datetime!(2024-08-21 5:57 pm -5),
        at: "weblog/thread-equivalence-checker-3.md",
    },
    Entry {
        slug: "complexity",
        title: "On Complexity",
        published_on: datetime!(2026-03-16 3:41 pm -7),
        at: "weblog/complexity.md",
    },
];
