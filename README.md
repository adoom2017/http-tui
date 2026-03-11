# http-tui

A terminal-based HTTP debugging tool (like Postman) written in Rust.

## Features

- **TUI interface** powered by `ratatui` + `crossterm` — runs in any terminal
- **3-panel layout** — Collection tree | Request editor | Response viewer
- **YAML collections** — Organize requests in `.yaml` files, nested in folders
- **Async HTTP** — Send requests without blocking the UI (powered by `reqwest` + `tokio`)
- **Cross-platform** — Windows, macOS, Linux (uses `rustls-tls`, no native TLS dependency)
- **JSON pretty-printing** with basic syntax coloring in response viewer
- **In-app help** — Press `?` at any time to show the full shortcut reference

## Installation

```bash
cargo build --release
# Binary at: target/release/http-tui (or http-tui.exe on Windows)
```

## Usage

Run from within a directory that has a `collections/` folder:

```bash
http-tui
```

## Collection Format

Create `.yaml` files inside a `collections/` folder. Subdirectories are supported:

```
collections/
  api/
    users.yaml
    auth.yaml
  examples/
    httpbin.yaml
```

Each `.yaml` file is a collection:

```yaml
name: My API Collection
requests:
  - name: Get Users
    method: GET
    url: https://api.example.com/users
    headers:
      Authorization: Bearer mytoken
      Accept: application/json

  - name: Create User
    method: POST
    url: https://api.example.com/users
    headers:
      Content-Type: application/json
    body: |
      {
        "name": "Alice",
        "email": "alice@example.com"
      }
```

Supported HTTP methods: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`

---

## Keyboard Shortcuts

> Press **`?`** inside the app to show/hide this reference as an overlay.

### Global (work everywhere)

| Key | Action |
|-----|--------|
| `Tab` | Focus next panel: Collections → Request → Response → Collections |
| `Shift+Tab` | Focus previous panel (reverse cycle) |
| `r` / `F5` | Send the current HTTP request |
| `Ctrl+S` | Save current request to YAML |
| `q` / `Ctrl+C` | Quit |
| `?` | Toggle keyboard shortcut help overlay |

---

### Collections Panel

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate tree items |
| `Enter` | Expand/collapse folder **or** select a request |
| `Home` / `End` | Jump to top / bottom of tree |
| `n` | New collection file inside the selected folder |
| `N` (Shift+N) | New folder inside the selected folder |

Collection files expand (▼/▶) to show individual requests as sub-items.  
Press **Enter** on a request to load it into the editor.

---

### Request Editor — Navigate Mode

| Key | Action |
|-----|--------|
| `↑` / `↓` | Move focus between URL bar and header rows |
| `←` (from URL bar) | Focus the Method box |
| `→` / `↓` (from Method box) | Return to URL bar |
| `Enter` | Start editing the focused field |
| `1` | Switch to **Headers** tab |
| `2` | Switch to **Body** tab |
| `m` / `M` | Cycle HTTP method forward / backward |
| `o` | Add a new header row |
| `d` | Delete selected header row |
| `n` | Create a new empty request |

**Method box** (when focused with `←`):

| Key | Action |
|-----|--------|
| `Enter` | Cycle to the next HTTP method |
| `m` / `M` | Cycle forward / backward |

---

### Request Editor — Editing Mode

Activated by pressing **Enter** on a field. Status bar shows **EDITING** in green.

| Key | Action |
|-----|--------|
| Type normally | Insert characters |
| `←` / `→` | Move cursor left / right |
| `Home` / `End` | Move cursor to start / end |
| `Backspace` / `Delete` | Delete character before / after cursor |
| `Ctrl+W` | Delete word backwards |
| `Ctrl+U` | Clear entire field |
| `Enter` *(URL field)* | Send the request immediately |
| `Tab` *(header field)* | Switch between Key ↔ Value column |
| `↑` / `↓` *(header field)* | Move to previous / next header row |
| `Esc` | Exit editing, return to Navigate mode |

---

### Response Panel

| Key | Action |
|-----|--------|
| `↑` / `↓` | Scroll one line |
| `PageUp` / `PageDown` | Scroll 10 lines |
| `Ctrl+U` / `Ctrl+D` | Scroll 10 lines up / down |
| `Home` / `End` | Scroll to top / bottom |
| `1` | Switch to **Body** tab |
| `2` | Switch to **Headers** tab |

