# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a personal website built in Rust using the Axum web framework. The site serves static content, markdown blog posts, and includes a simple weblog system.

## Architecture

- **Main Application**: `src/main.rs` contains the entire web server implementation
- **Static Assets**: Embedded using `rust-embed` from the `static/` directory  
- **Content**: Markdown files in `data/` directory, with blog posts in `data/weblog/`
- **HTML Generation**: Uses `maud` for compile-time HTML templating
- **Routing**: Axum-based router with static file serving and dynamic blog routes

### Key Components

- `Site`: Main router and homepage handler
- `StaticFiles`: Serves embedded static assets from `/static/*` routes
- `Weblog`: Blog system with entry listing and individual post pages
- `Entry`: Represents individual blog posts with metadata
- `Page`: HTML page structure and common elements

### Blog System

Blog entries are statically defined in the `WEBLOG_ENTRIES` array in `main.rs`. Each entry includes:
- Slug for URL routing
- Title and publish date
- Path to markdown file in `data/weblog/`

Routes are dynamically generated for each blog entry at compile time.

## Development Commands

### Build and Run
```bash
cargo build          # Build the project
cargo run            # Run the development server (localhost:3030)
cargo build --release # Build optimized release version
```

### Development Workflow
- Server runs on `http://127.0.0.1:3030`
- Static files are served from `/static/*` 
- Blog posts are at `/weblog` (index) and `/weblog/{slug}` (individual posts)
- Content changes in `data/` require recompilation due to `rust-embed`

### Adding Blog Posts

1. Create markdown file in `data/weblog/`
2. Add entry to `WEBLOG_ENTRIES` array in `main.rs` with:
   - Unique slug
   - Title
   - Publication date using `datetime!` macro
   - Path relative to `data/` directory

### Key Dependencies

- `axum`: Web framework for routing and HTTP handling
- `maud`: Compile-time HTML templating
- `rust-embed`: Static asset embedding
- `pulldown-cmark`: CommonMark-compliant Markdown parsing with extensions
- `tokio`: Async runtime
- `time`: Date/time handling
- `mime_guess`: MIME type detection for static files

### Markdown Processing

The project uses `pulldown-cmark` for markdown processing with strikethrough extension enabled. The markdown rendering pipeline:

1. Load content from embedded `data/` files
2. Parse with `pulldown_cmark::Parser` with extensions
3. Transform through `TextMergeStream`
4. Render to HTML with `pulldown_cmark::html::push_html`
5. Embed in page template using `PreEscaped` markup

### Project Structure

```
/
├── src/main.rs          # Complete application logic
├── static/              # Static assets (CSS, images, etc.)
│   └── style.css        # Main stylesheet
├── data/                # Content files
│   ├── home.md          # Homepage content
│   └── weblog/          # Blog post markdown files
├── Cargo.toml           # Dependencies and project config
└── CLAUDE.md            # This file
```

## Development Guidelines

- Always prioritize the _simplest_ and _cleanest_ solution
- Follow existing code patterns and conventions
- Content changes require recompilation due to embedded assets
- Blog entries must be added to both filesystem and `WEBLOG_ENTRIES` array