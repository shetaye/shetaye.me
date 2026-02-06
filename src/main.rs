use rust_embed::Embed;
use maud::{DOCTYPE, html, Markup, PreEscaped};
use axum::{
    extract::Path,
    http::{StatusCode, HeaderMap, header},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use time::macros::datetime;
use mime_guess::from_path;
use std::net::SocketAddr;
use tokio;
use pulldown_cmark::{Event, Tag};
use rand::Rng;
use futures::stream::unfold;
use axum::body::Body;

#[derive(Embed)]
#[folder = "data/"]
struct Data;

#[derive(Embed)]
#[folder = "static/"]
struct Static;

impl Data {
    fn get_str(file_path: &str) -> String {
        let buf = Data::get(file_path).unwrap();
        let s = match std::str::from_utf8(buf.data.as_ref()) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        return s.to_string();
    }
}


struct StaticFiles {}
impl StaticFiles {
    fn find(path: &str) -> Option<String> {
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

    fn routes() -> Router {
        Router::new().route("/static/{*path}", get(Self::get_reply_for))
    }
}

struct Common {}
impl Common {
    fn skeleton(head: Markup, body: Markup) -> Markup {
        html! {
            (DOCTYPE)
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1.0";
            head { (head) }
            body { (body) }
        }
    }
    fn includes(title: &str, additional: Option<Markup>) -> Markup {
        let base = html! {
            link rel="preload" href=(StaticFiles::find("style.css").unwrap()) as="style";
            link rel="preload" href=(StaticFiles::find("InterTight.ttf").unwrap()) as="font" type="font/ttf" crossorigin="anonymous";
            link rel="stylesheet" href=(StaticFiles::find("style.css").unwrap());
            link rel="icon" type="image/x-icon" href="/static/favicon.ico";
            link rel="icon" type="image/png" sizes="32x32" href="/static/favicon-32x32.png";
            link rel="icon" type="image/png" sizes="16x16" href="/static/favicon-16x16.png";
            link rel="apple-touch-icon" sizes="180x180" href="/static/apple-touch-icon.png";
            link rel="icon" type="image/png" sizes="192x192" href="/static/android-chrome-192x192.png";
            link rel="icon" type="image/png" sizes="512x512" href="/static/android-chrome-512x512.png";
            title { (title) }
        };

        // for some reason, using the built-in maud if-let doesn't work!
        if let Some(additional) = additional {
            html! {
                (additional)
                (base)
            }
        } else {
            base
        }
    }
    fn header() -> Markup {
        html! {
            nav {
                h1 { a href=(Site::find_home()) { "shetaye.me" }}
                a href=(Weblog::find_all()) { "weblog" }
                a href=(Site::find_work()) { "work" }
            }
        }
    }
    fn basic(title: &str, body: Markup) -> Markup {
        Self::skeleton(Self::includes(title, None), html! {
            (Self::header())
            (body)
        })
    }
}

fn render_url(dest_url: &str) -> String {
    if let Some(stripped) = dest_url.strip_prefix("weblog://") {
        if let Some(url) = Weblog::find(stripped) {
            return url;
        }
    }
    if let Some(stripped) = dest_url.strip_prefix("static://") {
        if let Some(url) = StaticFiles::find(stripped) {
            return url;
        }
    }
    dest_url.to_string()
}

fn render_markdown(markdown: &str) -> String {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    let parser = pulldown_cmark::Parser::new_ext(markdown, options);
    let iterator = pulldown_cmark::TextMergeStream::new(parser);

    let transformed = iterator.map(|event| {
        match event {
            Event::Start(Tag::Link { dest_url, link_type, title, id }) => {
                let new_dest_url = render_url(&dest_url).to_string().into();
                Event::Start(Tag::Link { dest_url: new_dest_url, link_type, title, id })
            }
            _ => event
        }
    });

    let mut rendered = String::new();
    pulldown_cmark::html::push_html(&mut rendered, transformed);

    rendered
}

/// A weblog entry
struct Entry {
    /// Slug
    slug: &'static str,
    /// Title
    title: &'static str,
    /// Publish datetime
    published_on: time::OffsetDateTime,
    /// Path in the data directory
    at: &'static str
}

impl Entry {
    async fn handler(&'static self) -> Html<String> {
        let content = Data::get_str(self.at);

        let rendered = render_markdown(content.as_str());
        
        let body = html! {
            article {
                h1 { (self.title) }
                span { (format!("{}", self.published_on.date())) }
                (PreEscaped(rendered))
            }
        };
        Html(Common::basic(self.title, body).into_string())
    }
}

static WEBLOG_ENTRIES: [Entry; 4] = [
    Entry {
        slug: "ls2j",
        title: "LS2J",
        published_on: datetime!(2023-12-14 4:45 pm -8),
        at: "weblog/ls2j.md"
    },
    Entry {
        slug: "thread-equivalence-checker",
        title: "Thread Equivalence Checking - Part 1",
        published_on: datetime!(2024-07-08 8:20 pm -7),
        at: "weblog/thread-equivalence-checker.md"
    },
    Entry {
        slug: "thread-equivalence-checker-2",
        title: "Thread Equivalence Checking - Part 2",
        published_on: datetime!(2024-08-19 4:57 pm +2),
        at: "weblog/thread-equivalence-checker-2.md"
    },
    Entry {
        slug: "thread-equivalence-checker-3",
        title: "Thread Equivalence Checking - Part 3",
        published_on: datetime!(2024-08-21 5:57 pm -5),
        at: "weblog/thread-equivalence-checker-3.md"
    }
];

struct Weblog {}
impl Weblog {
    fn find(slug: &str) -> Option<String> {
        WEBLOG_ENTRIES.iter().find(|entry| entry.slug == slug)
            .map(|_| format!("/weblog/{}", slug))
    }

    fn find_all() -> String { "/weblog".to_string() }

    fn entries_by_date() -> impl Iterator<Item = &'static Entry> {
        let mut entries: Vec<&Entry> = WEBLOG_ENTRIES.iter().collect();
        entries.sort_by(|a, b| b.published_on.cmp(&a.published_on));
        entries.into_iter()
    }

    /// Common listing all weblogs
    fn all() -> Markup {
        Common::basic("Weblog", html! {
            p { "all of my blog entries" }
            table {
                tr {
                    th { "Published On" }
                    th { "Post" }
                }
                @for entry in Self::entries_by_date() {
                    tr {
                        td { (entry.published_on.date().to_string()) }
                        td { a href=(Weblog::find(entry.slug).unwrap()) { (entry.title) }}
                    }
                }
            }
        })
    }

    async fn handler() -> Html<String> {
        Html(Self::all().into_string())
    }

    fn routes() -> Router {
        let mut router = Router::new().route("/weblog", get(Self::handler));
        
        for entry in WEBLOG_ENTRIES.iter() {
            let route_path = format!("/weblog/{}", entry.slug);
            router = router.route(&route_path, get(move || async move {
                entry.handler().await
            }));
        }
        
        router
    }
}

struct Faucet {}
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

    fn routes() -> Router {
        Router::new().route("/faucet/{size}", get(Self::handler))
    }
}

struct DesignLanguage {}
impl DesignLanguage {
    fn find() -> String { "/design-language".to_string() }

    async fn handler() -> Html<String> {
        let body = html! {
            article {
                h1 { "Design Language" }
                p { "A living reference of this site's visual components. Everything on this page is styled by the same stylesheet used across the entire site." }

                hr;

                // Typography
                h2 { "Typography" }
                h1 { "Heading 1" }
                h2 { "Heading 2" }
                h3 { "Heading 3" }
                h4 { "Heading 4" }
                h5 { "Heading 5" }
                h6 { "Heading 6" }
                p { "This is a paragraph of body text. It demonstrates the default font, size, line-height, and color used throughout the site." }
                p {
                    strong { "Bold text" } " — "
                    em { "Italic text" } " — "
                    s { "Strikethrough text" } " — "
                    code { "inline code" }
                }

                hr;

                // Fonts
                h2 { "Fonts" }
                h3 { "Inter Tight (sans-serif)" }
                p style="font-weight: 300;" { "Light (300) — The quick brown fox jumps over the lazy dog." }
                p style="font-weight: 400;" { "Regular (400) — The quick brown fox jumps over the lazy dog." }
                p style="font-weight: 500;" { "Medium (500) — The quick brown fox jumps over the lazy dog." }
                p style="font-weight: 700;" { "Bold (700) — The quick brown fox jumps over the lazy dog." }
                p style="font-weight: 900;" { "Black (900) — The quick brown fox jumps over the lazy dog." }
                p style="font-style: italic;" { "Italic — The quick brown fox jumps over the lazy dog." }

                h3 { "Source Code Pro (monospace)" }
                p style="font-family: 'Source Code Pro', Consolas, Monaco, monospace; font-weight: 400;" {
                    "Regular (400) — The quick brown fox jumps over the lazy dog."
                }
                p style="font-family: 'Source Code Pro', Consolas, Monaco, monospace; font-weight: 700;" {
                    "Bold (700) — The quick brown fox jumps over the lazy dog."
                }
                p style="font-family: 'Source Code Pro', Consolas, Monaco, monospace; font-style: italic;" {
                    "Italic — The quick brown fox jumps over the lazy dog."
                }

                hr;

                // Colors
                h2 { "Colors" }
                p { "The site uses a minimal palette that inverts in dark mode." }
                div style="display: flex; flex-wrap: wrap; gap: 1rem; margin: 1rem 0;" {
                    div style="width: 120px; text-align: center;" {
                        div style="width: 120px; height: 60px; background: #ffffff; border: 1px solid #ddd;" {}
                        p { "White (#fff)" }
                    }
                    div style="width: 120px; text-align: center;" {
                        div style="width: 120px; height: 60px; background: #000000; border: 1px solid #333;" {}
                        p { "Black (#000)" }
                    }
                    div style="width: 120px; text-align: center;" {
                        div style="width: 120px; height: 60px; background: #dddddd; border: 1px solid #ddd;" {}
                        p { "Border (#ddd)" }
                    }
                    div style="width: 120px; text-align: center;" {
                        div style="width: 120px; height: 60px; background: #333333; border: 1px solid #333;" {}
                        p { "Border dark (#333)" }
                    }
                }

                hr;

                // Links
                h2 { "Links" }
                p {
                    "This is a " a href=(DesignLanguage::find()) { "regular link" }
                    ". Links are underlined by default and the underline disappears on hover."
                }

                hr;

                // Lists
                h2 { "Lists" }
                h3 { "Unordered list" }
                ul {
                    li { "First item" }
                    li { "Second item" }
                    li {
                        "Third item with nested list"
                        ul {
                            li { "Nested item A" }
                            li { "Nested item B" }
                        }
                    }
                }
                h3 { "Ordered list" }
                ol {
                    li { "First item" }
                    li { "Second item" }
                    li {
                        "Third item with nested list"
                        ol {
                            li { "Nested item 1" }
                            li { "Nested item 2" }
                        }
                    }
                }

                hr;

                // Tables
                h2 { "Tables" }
                table {
                    tr {
                        th { "Header 1" }
                        th { "Header 2" }
                        th { "Header 3" }
                    }
                    tr {
                        td { "Row 1, Col 1" }
                        td { "Row 1, Col 2" }
                        td { "Row 1, Col 3" }
                    }
                    tr {
                        td { "Row 2, Col 1" }
                        td { "Row 2, Col 2" }
                        td { "Row 2, Col 3" }
                    }
                }

                hr;

                // Blockquotes
                h2 { "Blockquotes" }
                blockquote {
                    p { "This is a blockquote. It has a left border and italic styling, useful for setting apart quoted material." }
                }

                hr;

                // Code Blocks
                h2 { "Code Blocks" }
                p { "Inline: " code { "let x = 42;" } }
                pre {
                    code {
                        "fn main() {\n    println!(\"Hello, world!\");\n}"
                    }
                }

                hr;

                // Horizontal Rules
                h2 { "Horizontal Rules" }
                p { "The dividers between each section on this page are horizontal rules." }
                hr;

                // Navigation
                h2 { "Navigation" }
                p { "The nav bar at the top of this page is the canonical navigation component. It features inverted colors (white text on black background in light mode, black text on white background in dark mode) and the underline disappears on hover, with the colors inverting." }
            }
        };
        Html(Common::basic("Design Language", body).into_string())
    }

    fn routes() -> Router {
        Router::new().route("/design-language", get(Self::handler))
    }
}

struct Site {}
impl Site {
    fn find_home() -> String {
        "/".to_string()
    }

    fn home() -> Markup {
        let body = html! {
            p { "I'm Joseph Shetaye, a fourth-year undergraduate computer science student at Stanford University." }
            p { "I work with operating systems & chips." }
        };
        return Common::basic("Home", body);
    }

    async fn home_handler() -> Html<String> {
        Html(Self::home().into_string())
    }

    fn find_work() -> String {
        "/work".to_string()
    }

    fn work() -> Markup {
        let content = Data::get_str("work.md");

        let rendered = render_markdown(content.as_str());
        
        let body = html! {
            article {
                (PreEscaped(rendered))
            }
        };
        Common::basic("Work", body)
    }

    async fn work_handler() -> Html<String> {
        Html(Self::work().into_string())
    }


    fn routes() -> Router {
        Router::new()
            .route("/", get(Self::home_handler))
            .route("/work", get(Self::work_handler))
            .merge(StaticFiles::routes())
            .merge(Weblog::routes())
            .merge(Faucet::routes())
            .merge(DesignLanguage::routes())
    }
}



#[tokio::main]
async fn main() {
    let app = Site::routes();
    
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3030".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid port number");
    
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid HOST or PORT");
    
    println!("Server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
