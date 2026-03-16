use maud::{DOCTYPE, html, Markup};

use crate::data::StaticFiles;
use crate::pages::home::Site;
use crate::pages::weblog::Weblog;

pub struct Common {}
impl Common {
    pub fn skeleton(head: Markup, body: Markup) -> Markup {
        html! {
            (DOCTYPE)
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1.0";
            head { (head) }
            body { (body) }
        }
    }
    pub fn includes(title: &str, additional: Option<Markup>) -> Markup {
        let base = html! {
            link rel="preload" href=(StaticFiles::find("style.css").unwrap()) as="style";
            link rel="preload" href=(StaticFiles::find("Lora-VariableFont_wght.ttf").unwrap()) as="font" type="font/ttf" crossorigin="anonymous";
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
    pub fn header() -> Markup {
        html! {
            nav {
                h1 { a class="nav-button-big" href=(Site::find_home()) { "shetaye.me" }}
                a class="nav-button" href=(Weblog::find_all()) { "weblog" }
                a class="nav-button" href=(Site::find_work()) { "work" }
            }
        }
    }

    pub fn footer() -> Markup {
        html! {
            footer class="site-footer" {
                "© " (time::OffsetDateTime::now_utc().year()) " Joseph Shetaye"
            }
        }
    }

    pub fn basic(title: &str, body: Markup) -> Markup {
        Self::skeleton(Self::includes(title, None), html! {
            (Self::header())
            (body)
            (Self::footer())
        })
    }
}
