use pulldown_cmark::{Event, Tag};

use crate::data::{Entry, StaticFiles};

fn render_url(dest_url: &str) -> String {
    if let Some(stripped) = dest_url.strip_prefix("weblog://") {
        if let Some(url) = Entry::find(stripped) {
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

pub fn render_markdown(markdown: &str) -> String {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    let parser = pulldown_cmark::Parser::new_ext(markdown, options);
    let iterator = pulldown_cmark::TextMergeStream::new(parser);

    let transformed = iterator.map(|event| match event {
        Event::Start(Tag::Link {
            dest_url,
            link_type,
            title,
            id,
        }) => {
            let new_dest_url = render_url(&dest_url).to_string().into();
            Event::Start(Tag::Link {
                dest_url: new_dest_url,
                link_type,
                title,
                id,
            })
        }
        _ => event,
    });

    let mut rendered = String::new();
    pulldown_cmark::html::push_html(&mut rendered, transformed);

    rendered
}
