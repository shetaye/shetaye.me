use maud::{html, Markup, PreEscaped};
use axum::{response::Html, routing::get, Router};

use crate::data::{Data, Entry, WEBLOG_ENTRIES};
use crate::markdown::render_markdown;
use crate::layout::Common;

impl Entry {
    pub async fn handler(&'static self) -> Html<String> {
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

pub struct Weblog {}
impl Weblog {
    pub fn find_all() -> String {
        "/weblog".to_string()
    }

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
                        td { a href=(Entry::find(entry.slug).unwrap()) { (entry.title) }}
                    }
                }
            }
        })
    }

    async fn handler() -> Html<String> {
        Html(Self::all().into_string())
    }

    pub fn routes() -> Router {
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
