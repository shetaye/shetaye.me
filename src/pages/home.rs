use maud::{html, Markup, PreEscaped};
use axum::{response::Html, routing::get, Router};

use crate::data::Data;
use crate::markdown::render_markdown;
use crate::layout::Common;

pub struct Site {}
impl Site {
    pub fn find_home() -> String {
        "/".to_string()
    }

    fn home() -> Markup {
        let content = Data::get_str("home.md");

        let rendered = render_markdown(content.as_str());

        let body = html! {
            article {
                (PreEscaped(rendered))
            }
        };
        Common::basic("Home", body)
    }

    async fn home_handler() -> Html<String> {
        Html(Self::home().into_string())
    }

    pub fn find_work() -> String {
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

    pub fn routes() -> Router {
        Router::new()
            .route("/", get(Self::home_handler))
            .route("/work", get(Self::work_handler))
    }
}
