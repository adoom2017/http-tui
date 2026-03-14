# Copilot Instructions for http-tui

## Project Overview
A terminal-based HTTP debugging tool (like Postman but TUI-only). Users organize HTTP requests in YAML collection files, send them asynchronously, and view responses — all within a 3-panel terminal layout.

## Build & Run Commands
```bash
cargo build            # Debug build
cargo build --release  # Release build
cargo run              # Run (uses ./collections/ as root)
cargo run -- /path     # Run with custom collections root
cargo check            # Fast syntax/type check
cargo fmt              # Format code
cargo clippy           # Lint
cargo test             # Tests (minimal test suite currently)
```

## Architecture

All application state lives in `App` (src/app.rs). The event loop in `main.rs` is the only place that mutates state from async results or dispatches HTTP tasks.

```
main.rs (tokio event loop, 16ms tick)
  ├── terminal.draw()      → ui::render(&mut App)
  ├── rx.try_recv()        → receive async HTTP responses via mpsc channel
  └── event::poll()        → events/handler.rs → AppAction
                              └── AppAction::SendRequest → tokio::spawn(http::send_request)
```

**Module responsibilities:**
- `app.rs` — All UI state + methods that mutate it (900 lines; the central hub)
- `events/handler.rs` — Keyboard/mouse → state mutations + returns `AppAction` (1059 lines)
- `ui/` — Stateless rendering; each component receives `&mut App` to record panel `Rect`s for mouse hit-testing
- `http/client.rs` — Single `send_request()` async fn using reqwest; spawned as a tokio task
- `storage/yaml.rs` — Recursive YAML load/save + builds `TreeNode` hierarchy
- `models/` — Plain data structs: `Request`, `Collection`, `AppResponse`, `Environment`

## Key Conventions

### State mutations belong in `app.rs`
Event handlers in `events/handler.rs` call `app.*` methods rather than mutating fields directly. Any non-trivial state change should be a method on `App`.

### UI components store their `Rect` for mouse hit-testing
`ui/mod.rs` passes `&mut App` (not `&App`) so renderers can write panel bounds (e.g., `app.collections_rect`, `app.request_rect`). Mouse events in `handler.rs` then use these rects to determine which panel was clicked.

### Two editing modes
`app.editing: bool` gates whether keypresses are routed to text input or navigation commands. Most key handlers check this flag first.

### Overlay/dialog state is encoded in `App` fields
Active overlays are checked in order in `ui::render()` — the last one drawn wins. Dialog state uses enums with a `None` variant (e.g., `CreateMode::None`, `ConfirmDelete::None`) rather than `Option<...>`.

### Environment variable substitution
`{{VAR}}` syntax in URLs, headers, and bodies. Substitution happens in `app.resolved_request()` just before spawning the HTTP task — the stored `Request` always holds the raw template.

### Error handling
- `anyhow::Result<T>` + `?` throughout all I/O paths
- Errors surfaced to the user via `app.status_message` (status bar), never panics

### Collection file format
YAML files under `collections/`. `env.yaml` in the root holds environment variables (git-ignored). Empty `headers` and `body: null` are skipped during serialization via `serde` attributes.

## Key Dependencies
| Crate | Role |
|---|---|
| `ratatui` 0.29 | TUI rendering |
| `crossterm` 0.28 | Terminal raw mode, events |
| `tokio` 1 (full) | Async runtime |
| `reqwest` 0.12 (rustls-tls) | HTTP client (no OpenSSL required) |
| `serde_yaml` 0.9 | Collection persistence |
| `serde_json` (preserve_order) | Response pretty-printing |
| `anyhow` 1 | Error context |
| `walkdir` 2 | Recursive collection discovery |
