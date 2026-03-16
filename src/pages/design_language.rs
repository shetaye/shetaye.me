use maud::html;
use axum::{response::Html, routing::get, Router};
use std::collections::HashMap;

use crate::layout::Common;
use crate::pages::home::Site;
use crate::pages::weblog::Weblog;

pub struct DesignLanguage {}
impl DesignLanguage {
    fn css_vars_with_prefix_from(css: &str, prefix: &str) -> Vec<(String, String)> {
        css.lines()
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

    fn css_vars_with_prefix(prefix: &str) -> Vec<(String, String)> {
        Self::css_vars_with_prefix_from(include_str!("../../static/input.css"), prefix)
    }

    fn css_vars_with_prefix_by_mode(prefix: &str) -> (Vec<(String, String)>, Vec<(String, String)>) {
        let css = include_str!("../../static/input.css");
        let Some((light_css, dark_css)) = css.split_once("@media (prefers-color-scheme: dark)") else {
            let vars = Self::css_vars_with_prefix_from(css, prefix);
            return (vars.clone(), vars);
        };

        let light = Self::css_vars_with_prefix_from(light_css, prefix);
        let dark_overrides = Self::css_vars_with_prefix_from(dark_css, prefix);
        let mut dark_map: HashMap<String, String> = light.iter().cloned().collect();
        for (name, value) in dark_overrides {
            dark_map.insert(name, value);
        }
        let dark = light
            .iter()
            .filter_map(|(name, _)| dark_map.get(name).map(|value| (name.clone(), value.clone())))
            .collect();
        (light, dark)
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
        let (modus_palette_light, modus_palette_dark) = Self::css_vars_with_prefix_by_mode("--modus-");
        let modus_palette_dark_map: HashMap<String, String> = modus_palette_dark.into_iter().collect();
        let modus_operandi_palette: Vec<(String, String, String)> = modus_palette_light
            .iter()
            .map(|(name, light_hex)| {
                let dark_hex = modus_palette_dark_map
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| light_hex.clone());
                (name.clone(), light_hex.clone(), dark_hex)
            })
            .collect();
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
        let guide_palette: Vec<(String, String, String)> = guide_palette_tokens
            .iter()
            .filter_map(|token| {
                modus_operandi_palette
                    .iter()
                    .find(|(name, _, _)| name == token)
                    .cloned()
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
			    a href="https://www.jemoka.com/design/" { "Jack's design" }
			" and "
			    a href="https://usgraphics.com/" { "Neil's design" }
			"."
		    }

                    h2 { "Color Palette" }
                    div class="palette-grid" {
                        @for (name, light_hex, dark_hex) in &guide_palette {
                            div
                                class="palette-swatch"
                                style={
                                    "--swatch-fg-light: " (Self::text_color_for_hex(light_hex)) "; "
                                    "--swatch-fg-dark: " (Self::text_color_for_hex(dark_hex)) "; "
                                    "background: var(--modus-" (name) ");"
                                } {
                                div class="palette-swatch-name" { (name) }
                                div class="palette-swatch-hex" {
                                    span class="palette-hex-light" { (light_hex) }
                                    span class="palette-hex-dark" { (dark_hex) }
                                }
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
                        p style="font-family: 'Inter Tight', system-ui, sans-serif; font-weight: 300;" { "Light (300) - The quick brown fox jumps over the lazy dog." }
                        p style="font-family: 'Inter Tight', system-ui, sans-serif; font-weight: 400;" { "Regular (400) - The quick brown fox jumps over the lazy dog." }
                        p style="font-family: 'Inter Tight', system-ui, sans-serif; font-weight: 500;" { "Medium (500) - The quick brown fox jumps over the lazy dog." }
                        p style="font-family: 'Inter Tight', system-ui, sans-serif; font-weight: 700;" { "Bold (700) - The quick brown fox jumps over the lazy dog." }
                        p style="font-family: 'Inter Tight', system-ui, sans-serif; font-weight: 900;" { "Black (900) - The quick brown fox jumps over the lazy dog." }
                        p style="font-family: 'Inter Tight', system-ui, sans-serif; font-style: italic;" { "Italic - The quick brown fox jumps over the lazy dog." }

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

                        h3 style="font-family: 'Lora', Georgia, 'Times New Roman', serif;" { "Lora (serif)" }
                        p style="font-family: 'Lora', Georgia, 'Times New Roman', serif; font-weight: 400;" {
                            "Regular (400) - The quick brown fox jumps over the lazy dog."
                        }
                        p style="font-family: 'Lora', Georgia, 'Times New Roman', serif; font-weight: 700;" {
                            "Bold (700) - The quick brown fox jumps over the lazy dog."
                        }
                        p style="font-family: 'Lora', Georgia, 'Times New Roman', serif; font-style: italic;" {
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
                        @for (name, light_hex, dark_hex) in &modus_operandi_palette {
                            div
                                class="palette-swatch"
                                style={
                                    "--swatch-fg-light: " (Self::text_color_for_hex(light_hex)) "; "
                                    "--swatch-fg-dark: " (Self::text_color_for_hex(dark_hex)) "; "
                                    "background: var(--modus-" (name) ");"
                                } {
                                div class="palette-swatch-name" { (name) }
                                div class="palette-swatch-hex" {
                                    span class="palette-hex-light" { (light_hex) }
                                    span class="palette-hex-dark" { (dark_hex) }
                                }
                            }
                        }
                    }
                }
            }
        };
        Html(Common::basic("Design Language", body).into_string())
    }

    pub fn routes() -> Router {
        Router::new().route("/design-language", get(Self::handler))
    }
}
