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
                h1 { a class="nav-button-big" href=(Site::find_home()) { "shetaye.me" }}
                a class="nav-button" href=(Weblog::find_all()) { "weblog" }
                a class="nav-button" href=(Site::find_work()) { "work" }
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
    fn css_vars_with_prefix(prefix: &str) -> Vec<(String, String)> {
        include_str!("../static/input.css")
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if !trimmed.starts_with(prefix) {
                    return None;
                }
                let (name, value) = trimmed.split_once(':')?;
                let token_name = name.trim().trim_start_matches(prefix).to_string();
                let token_value = value.trim().trim_end_matches(';').to_string();
                Some((token_name, token_value))
            })
            .collect()
    }

    fn modus_operandi_palette() -> Vec<(String, String)> {
        Self::css_vars_with_prefix("--modus-")
    }

    fn spacing_scale() -> Vec<(String, String)> {
        let mut scale = Self::css_vars_with_prefix("--space-");
        scale.sort_by_key(|(name, _)| name.parse::<u32>().ok().unwrap_or(u32::MAX));
        scale
    }

    fn text_color_for_hex(hex: &str) -> &'static str {
        let clean = hex.trim().trim_start_matches('#');
        if clean.len() != 6 {
            return "#000000";
        }
        let parse = |start: usize| u8::from_str_radix(&clean[start..start + 2], 16).ok();
        let (Some(r), Some(g), Some(b)) = (parse(0), parse(2), parse(4)) else {
            return "#000000";
        };
        let yiq = (r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000;
        if yiq >= 140 { "#000000" } else { "#ffffff" }
    }

    async fn handler() -> Html<String> {
        let modus_operandi_palette = Self::modus_operandi_palette();
        let spacing_scale = Self::spacing_scale();
        let guide_palette_tokens = [
            "bg-main",
            "bg-dim",
            "fg-main",
            "fg-dim",
            "fg-alt",
            "bg-active",
            "bg-inactive",
            "border",
        ];
        let guide_palette: Vec<(String, String)> = guide_palette_tokens
            .iter()
            .filter_map(|token| {
                modus_operandi_palette
                    .iter()
                    .find(|(name, _)| name == token)
                    .map(|(_, hex)| ((*token).to_string(), hex.to_string()))
            })
            .collect();

        let body = html! {
            article class="guide-shell" {
                div class="guide-top" {
                    h1 class="guide-title" {
                        span class="guide-badge" { "A" }
                        "Article Design Language"
                    }
                }

                div class="guide-divider" {}

                div class="guide-main" {
                    p {
                        "Single-column system for article-heavy pages: dense but readable text, compact controls, "
                        "and restrained accents for navigation, actions, and status."
                    }

		    p {
			"Based on "
			    a href="https://github.com/shetaye/funny-moka/" { "Jack's design" }
			" and "
			    a href="https://usgraphics.com/" { "Neil's design" }
			"."
		    }

                    h2 { "Color Palette" }
                    div class="palette-grid" {
                        @for (name, hex) in &guide_palette {
                            div
                                class="palette-swatch"
                                style={ "background: var(--modus-" (name) "); color: " (Self::text_color_for_hex(hex)) ";" } {
                                div class="palette-swatch-name" { (name) }
                                div class="palette-swatch-hex" { (hex) }
                            }
                        }
                    }

                    h2 { "Typography" }
                    div class="guide-sample" {
                        h1 { "Heading 1" }
                        h2 { "Heading 2" }
                        h3 { "Heading 3" }
                        h4 { "Heading 4" }
                        h5 { "Heading 5" }
                        h6 { "Heading 6" }
                        p { "This is a paragraph of body text. It demonstrates the default font, size, line-height, and color used throughout the site." }
                        p {
                            strong { "Bold text" } " - "
                            em { "Italic text" } " - "
                            s { "Strikethrough text" } " - "
                            code { "inline code" }
                        }

                        h3 style="font-family: 'Inter Tight', system-ui, sans-serif;" { "Inter Tight (sans-serif)" }
                        p style="font-weight: 300;" { "Light (300) - The quick brown fox jumps over the lazy dog." }
                        p style="font-weight: 400;" { "Regular (400) - The quick brown fox jumps over the lazy dog." }
                        p style="font-weight: 500;" { "Medium (500) - The quick brown fox jumps over the lazy dog." }
                        p style="font-weight: 700;" { "Bold (700) - The quick brown fox jumps over the lazy dog." }
                        p style="font-weight: 900;" { "Black (900) - The quick brown fox jumps over the lazy dog." }
                        p style="font-style: italic;" { "Italic - The quick brown fox jumps over the lazy dog." }

                        h3 style="font-family: 'Source Code Pro', Consolas, Monaco, monospace;" { "Source Code Pro (monospace)" }
                        p style="font-family: 'Source Code Pro', Consolas, Monaco, monospace; font-weight: 400;" {
                            "Regular (400) - The quick brown fox jumps over the lazy dog."
                        }
                        p style="font-family: 'Source Code Pro', Consolas, Monaco, monospace; font-weight: 700;" {
                            "Bold (700) - The quick brown fox jumps over the lazy dog."
                        }
                        p style="font-family: 'Source Code Pro', Consolas, Monaco, monospace; font-style: italic;" {
                            "Italic - The quick brown fox jumps over the lazy dog."
                        }

                        h3 style="font-family: 'IBM Plex Serif', Georgia, 'Times New Roman', serif;" { "IBM Plex Serif (serif)" }
                        p style="font-family: 'IBM Plex Serif', Georgia, 'Times New Roman', serif; font-weight: 400;" {
                            "Regular (400) - The quick brown fox jumps over the lazy dog."
                        }
                        p style="font-family: 'IBM Plex Serif', Georgia, 'Times New Roman', serif; font-weight: 700;" {
                            "Bold (700) - The quick brown fox jumps over the lazy dog."
                        }
                        p style="font-family: 'IBM Plex Serif', Georgia, 'Times New Roman', serif; font-style: italic;" {
                            "Italic - The quick brown fox jumps over the lazy dog."
                        }
                    }

                    h2 { "Buttons" }
                    p {
                        a class="button" href=(Weblog::find_all()) { "button" } " "
                        a class="button-big" href=(Site::find_home()) { "button-big" }
                    }
                    p {
                        a class="nav-button" href=(Weblog::find_all()) { "nav-button" } " "
                        a class="nav-button-big" href=(Site::find_home()) { "nav-button-big" }
                    }

                    h2 { "Component Reference" }
                    table class="guide-reference-table" {
                        tr {
                            th { "Component" }
                            th { "Description" }
                        }
                        tr { td { "Hero title" } td { "Single, high-contrast entrypoint heading." } }
                        tr { td { "Meta row" } td { "Date, read time, and topic chips." } }
                        tr { td { "Callout card" } td { "Accent-backed emphasis block." } }
                        tr { td { "Code block" } td { "Monospace section with subtle surface shift." } }
                        tr { td { "Inline note" } td { "Compact annotation inside article flow." } }
                    }

                    h2 { "States" }
                    div class="guide-states" {
                        div class="guide-state" { "Default" }
                        div class="guide-state guide-state-hover" { "Hover" }
                        div class="guide-state guide-state-focus" { "Focused" }
                        div class="guide-state guide-state-selected" { "Selected" }
                        div class="guide-state guide-state-warning" { "Warning" }
                        div class="guide-state guide-state-error" { "Error" }
                    }

                    h2 { "Notifications" }
                    div class="guide-notifications" {
                        div class="guide-note guide-note-info" { "Article saved successfully." }
                        div class="guide-note guide-note-warning" { "This draft has unresolved references." }
                        div class="guide-note guide-note-error" { "Publishing failed: missing required metadata." }
                    }

                    h2 { "Spacing Scale" }
                    div class="guide-spacing" {
                        @for (token, value) in &spacing_scale {
                            div class="guide-space-item" {
                                div class="guide-space-bar" style={ "width: var(--space-" (token) ");" } {}
                                span { "--space-" (token) ": " (value) }
                            }
                        }
                    }

                    h2 { "Modus Operandi Reference Swatches" }
                    p { "Dense swatch view for contrast checks using shared CSS variables." }
                    div class="palette-grid" {
                        @for (name, hex) in modus_operandi_palette {
                            div
                                class="palette-swatch"
                                style={ "background: var(--modus-" (name) "); color: " (Self::text_color_for_hex(&hex)) ";" } {
                                div class="palette-swatch-name" { (name) }
                                div class="palette-swatch-hex" { (hex) }
                            }
                        }
                    }
                }
            }
        };
        Html(Common::basic("Design Language", body).into_string())
    }

    fn find() -> String {
        "/design-language".to_string()
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

	    h1 { "Other People" }

	    p { "My girlfriend "
		 a href="https://skylarstrudwick.com" { "Skylar" }
		 " is an excellent human rights researcher and advocate, and she has several blogs and podcasts on the topic! You should check her out."
	    }

	    p { "I've also met many amazing people at and around Stanford. "
		 a href="https://jemoka.com" { "Jack" }
		 ", "
		 a href="https://kc3wny" { "Mason" }
		 ", and "
		 a href="https://kdrag0n.dev" { "Danny" }
		 " are a few"
	    }

	    h1 { "This webpage" }
            p {
                "I recently developed a personal design language, which this website follows. It can be found "
                a href=(DesignLanguage::find()) { "here" }
                "."
            }
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
