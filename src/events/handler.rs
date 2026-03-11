use crate::app::{App, ConfirmDelete, CreateMode, EditorField, EnvRow, FocusPanel, HeaderRow, RequestTab, ResponseTab};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

/// Actions sent to main loop for async operations
#[derive(Debug, PartialEq)]
pub enum AppAction {
    SendRequest,
    Quit,
    None,
}

/// Handle a key event and mutate app state.
/// Returns an AppAction for async side effects.
pub fn handle_key(app: &mut App, key: KeyEvent) -> AppAction {
    // Global shortcuts that always work
    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => return AppAction::Quit,
        // Direct panel jump: W=Collections  E=Request Editor  V=Response Viewer
        (KeyCode::Char('W'), _) if !app.editing => {
            app.focus = FocusPanel::CollectionTree;
            return AppAction::None;
        }
        (KeyCode::Char('E'), _) if !app.editing => {
            app.focus = FocusPanel::RequestEditor;
            return AppAction::None;
        }
        (KeyCode::Char('V'), _) if !app.editing => {
            app.focus = FocusPanel::ResponseViewer;
            return AppAction::None;
        }
        // Ctrl+O — open change-root dialog (global, works from anywhere)
        (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
            app.show_dir_input = !app.show_dir_input;
            if app.show_dir_input {
                app.open_dir_input();
                app.show_help = false;
                app.show_env_editor = false;
            }
            return AppAction::None;
        }
        // Toggle help overlay — ? closes editing too
        (KeyCode::Char('?'), _) if !app.show_env_editor && !app.show_dir_input => {
            app.show_help = !app.show_help;
            app.editing = false;
            return AppAction::None;
        }
        (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
            if app.show_env_editor {
                app.save_env();
            } else if let Err(e) = app.save_current_request() {
                app.status_message = format!("Error saving: {e}");
            }
            return AppAction::None;
        }
        // Toggle env editor (e key, not while editing text)
        (KeyCode::Char('e'), KeyModifiers::NONE) if !app.editing && !app.show_dir_input => {
            app.show_env_editor = !app.show_env_editor;
            app.show_help = false;
            app.editing = false;
            return AppAction::None;
        }
        (KeyCode::F(5), _) if !app.show_env_editor && !app.show_dir_input => {
            app.is_loading = true;
            app.status_message = "Sending request...".to_string();
            return AppAction::SendRequest;
        }
        _ => {}
    }

    // If dir input dialog is open, route all input there
    if app.show_dir_input {
        return handle_dir_input(app, key);
    }

    // If help is showing, any other key closes it
    if app.show_help {
        app.show_help = false;
        return AppAction::None;
    }

    // If env editor is open, route all input there
    if app.show_env_editor {
        return handle_env_input(app, key);
    }

    // If delete confirmation is open, route there
    if app.confirm_delete.is_active() {
        return handle_confirm_delete(app, key);
    }

    // If a create dialog is open, route all input there
    if app.create_mode.is_active() {
        return handle_create_input(app, key);
    }

    if app.editing {
        handle_text_input(app, key)
    } else {
        handle_navigation(app, key)
    }
}

// ---------------------------------------------------------------------------
// Navigation (editing = false)
// ---------------------------------------------------------------------------

fn handle_navigation(app: &mut App, key: KeyEvent) -> AppAction {
    match app.focus {
        FocusPanel::CollectionTree => handle_nav_tree(app, key),
        FocusPanel::RequestEditor => handle_nav_editor(app, key),
        FocusPanel::ResponseViewer => handle_nav_response(app, key),
    }
}

fn handle_nav_tree(app: &mut App, key: KeyEvent) -> AppAction {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) => return AppAction::Quit,

        // Panel cycling with Tab
        (KeyCode::Tab, KeyModifiers::NONE) => {
            app.editor_field = EditorField::Url;
            app.focus = FocusPanel::RequestEditor;
        }
        (KeyCode::BackTab, _) => {
            app.focus = FocusPanel::ResponseViewer;
        }

        // Navigate tree with arrow keys
        (KeyCode::Down, _) => app.tree_navigate_down(),
        (KeyCode::Up, _) => app.tree_navigate_up(),

        // Jump to top/bottom
        (KeyCode::Home, _) => app.tree_selected = 0,
        (KeyCode::End, _) => {
            let len = app.tree_items.len();
            if len > 0 { app.tree_selected = len - 1; }
        }

        // Select/expand
        (KeyCode::Enter, _) => {
            app.tree_select_item();
        }
        // Create new collection file — inside the selected directory
        (KeyCode::Char('n'), KeyModifiers::NONE) => {
            let parent_path = app.get_create_parent_path();
            app.create_mode = CreateMode::Collection {
                input: String::new(),
                cursor: 0,
                parent_path,
            };
        }
        // Create new folder (Shift+N) — at the same level as the selected item
        (KeyCode::Char('N'), _) => {
            let parent_path = app.get_sibling_parent_path();
            app.create_mode = CreateMode::Folder {
                input: String::new(),
                cursor: 0,
                parent_path,
            };
        }
        // Add new request to selected collection file
        (KeyCode::Char('a'), KeyModifiers::NONE) => {
            app.add_new_request_to_selected();
        }
        // Delete selected item (request: immediate; file/folder: confirm first)
        (KeyCode::Char('D'), _) => {
            app.prompt_delete_selected();
        }
        // Refresh collections from disk
        (KeyCode::Char('R'), _) | (KeyCode::F(5), _) => {
            app.reload_collections();
            app.status_message = "Refreshed".to_string();
        }
        _ => {}
    }
    AppAction::None
}

fn handle_nav_editor(app: &mut App, key: KeyEvent) -> AppAction {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) => return AppAction::Quit,

        // Panel cycling with Tab
        (KeyCode::Tab, KeyModifiers::NONE) => {
            app.focus = FocusPanel::ResponseViewer;
        }
        (KeyCode::BackTab, _) => {
            app.focus = FocusPanel::CollectionTree;
        }

        // Send request
        (KeyCode::Char('r'), KeyModifiers::NONE) => {
            app.is_loading = true;
            app.status_message = "Sending request...".to_string();
            return AppAction::SendRequest;
        }

        // Method cycling
        (KeyCode::Char('m'), KeyModifiers::NONE) => {
            app.next_method();
            app.status_message = format!("Method: {}", app.current_request.method);
        }
        (KeyCode::Char('M'), _) => {
            app.prev_method();
            app.status_message = format!("Method: {}", app.current_request.method);
        }

        // Sub-tab switching
        (KeyCode::Char('1'), KeyModifiers::NONE) => {
            app.request_tab = RequestTab::Headers;
            app.editor_field = EditorField::Headers;
        }
        (KeyCode::Char('2'), KeyModifiers::NONE) => {
            app.request_tab = RequestTab::Body;
            app.editor_field = EditorField::Body;
        }

        // Arrow keys: navigate header rows (↑ on first row goes to URL bar)
        // Left/Right navigate between Method box and URL bar on the top row
        (KeyCode::Left, _) if app.editor_field == EditorField::Url => {
            app.editor_field = EditorField::Method;
        }
        (KeyCode::Right, _) if app.editor_field == EditorField::Method => {
            app.editor_field = EditorField::Url;
        }
        (KeyCode::Down, _) => {
            if app.editor_field == EditorField::Name {
                app.editor_field = EditorField::Url;
            } else if app.editor_field == EditorField::Method {
                // ↓ from Method goes to URL
                app.editor_field = EditorField::Url;
            } else if app.request_tab == RequestTab::Headers {
                if app.editor_field == EditorField::Url {
                    app.editor_field = EditorField::Headers;
                } else if !app.header_rows.is_empty()
                    && app.header_selected + 1 < app.header_rows.len()
                {
                    app.header_selected += 1;
                }
            } else if app.request_tab == RequestTab::Body {
                app.editor_field = EditorField::Body;
            }
        }
        (KeyCode::Up, _) => {
            if app.request_tab == RequestTab::Headers {
                if app.editor_field == EditorField::Url {
                    app.editor_field = EditorField::Name;
                } else if app.editor_field == EditorField::Headers && app.header_selected == 0 {
                    app.editor_field = EditorField::Url;
                } else if app.editor_field == EditorField::Headers && app.header_selected > 0 {
                    app.header_selected -= 1;
                }
            } else if app.editor_field == EditorField::Url {
                app.editor_field = EditorField::Name;
            }
        }

        // Enter: activate editing for focused field; on Method box, cycle through methods
        (KeyCode::Enter, _) if app.editor_field == EditorField::Name => {
            app.editing = true;
        }
        (KeyCode::Enter, _) if app.editor_field == EditorField::Method => {
            app.next_method();
            app.status_message = format!("Method: {}", app.current_request.method);
        }
        (KeyCode::Enter, _) if app.editor_field == EditorField::Url => {
            app.editing = true;
        }
        (KeyCode::Enter, _) if app.editor_field == EditorField::Headers => {
            app.header_cursor = app
                .header_rows
                .get(app.header_selected)
                .map(|r| if app.header_edit_key { r.key.len() } else { r.value.len() })
                .unwrap_or(0);
            app.editing = true;
        }
        (KeyCode::Enter, _) if app.editor_field == EditorField::Body => {
            app.editing = true;
        }
        // Default Enter: activate URL editing
        (KeyCode::Enter, _) => {
            app.editor_field = EditorField::Url;
            app.editing = true;
        }

        // Add new header row
        (KeyCode::Char('o'), KeyModifiers::NONE) => {
            if app.request_tab == RequestTab::Headers {
                app.header_rows.push(HeaderRow { key: String::new(), value: String::new() });
                app.header_selected = app.header_rows.len() - 1;
                app.header_edit_key = true;
                app.header_cursor = 0;
                app.editor_field = EditorField::Headers;
                app.editing = true;
            }
        }
        // Delete selected header row
        (KeyCode::Char('d'), KeyModifiers::NONE) => {
            if app.request_tab == RequestTab::Headers && !app.header_rows.is_empty() {
                app.header_rows.remove(app.header_selected);
                if app.header_selected >= app.header_rows.len() && app.header_selected > 0 {
                    app.header_selected -= 1;
                }
                app.status_message = "Header deleted".to_string();
            }
        }
        // New request
        (KeyCode::Char('n'), KeyModifiers::NONE) => {
            app.current_request = crate::models::Request::new("New Request");
            app.current_request_index = None;
            app.header_rows.clear();
            app.url_cursor = 0;
            app.body_cursor = 0;
            app.response = None;
            app.status_message = "New request".to_string();
        }
        _ => {}
    }
    AppAction::None
}

fn handle_nav_response(app: &mut App, key: KeyEvent) -> AppAction {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), KeyModifiers::NONE) => return AppAction::Quit,

        // Panel cycling with Tab
        (KeyCode::Tab, KeyModifiers::NONE) => {
            app.focus = FocusPanel::CollectionTree;
        }
        (KeyCode::BackTab, _) => {
            app.focus = FocusPanel::RequestEditor;
        }

        // Vertical scroll
        (KeyCode::Down, _) => app.response_scroll = app.response_scroll.saturating_add(1),
        (KeyCode::Up, _) => app.response_scroll = app.response_scroll.saturating_sub(1),
        (KeyCode::PageDown, _) | (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
            app.response_scroll = app.response_scroll.saturating_add(10);
        }
        (KeyCode::PageUp, _) | (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
            app.response_scroll = app.response_scroll.saturating_sub(10);
        }
        (KeyCode::Home, _) => { app.response_scroll = 0; app.response_scroll_x = 0; }
        (KeyCode::End, _) => app.response_scroll = app.response_scroll.saturating_add(9999),

        // Horizontal scroll
        (KeyCode::Right, _) => app.response_scroll_x = app.response_scroll_x.saturating_add(4),
        (KeyCode::Left, _) => app.response_scroll_x = app.response_scroll_x.saturating_sub(4),
        (KeyCode::Char('l'), KeyModifiers::NONE) => app.response_scroll_x = app.response_scroll_x.saturating_add(4),
        (KeyCode::Char('h'), KeyModifiers::NONE) => app.response_scroll_x = app.response_scroll_x.saturating_sub(4),

        // Sub-tab switching
        (KeyCode::Char('1'), KeyModifiers::NONE) => {
            app.response_tab = ResponseTab::Body;
            app.response_scroll = 0;
            app.response_scroll_x = 0;
        }
        (KeyCode::Char('2'), KeyModifiers::NONE) => {
            app.response_tab = ResponseTab::Headers;
            app.response_scroll = 0;
            app.response_scroll_x = 0;
        }
        _ => {}
    }
    AppAction::None
}

// ---------------------------------------------------------------------------
// Create dialog input
// ---------------------------------------------------------------------------

fn handle_create_input(app: &mut App, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Esc => {
            app.create_mode = CreateMode::None;
        }
        KeyCode::Enter => {
            app.confirm_create();
        }
        KeyCode::Char(c)
            if key.modifiers == KeyModifiers::NONE
                || key.modifiers == KeyModifiers::SHIFT =>
        {
            if let Some((input, cursor)) = app.create_mode.input_mut() {
                input.insert(*cursor, c);
                *cursor += c.len_utf8();
            }
        }
        KeyCode::Backspace => {
            if let Some((input, cursor)) = app.create_mode.input_mut() {
                if *cursor > 0 {
                    *cursor -= 1;
                    input.remove(*cursor);
                }
            }
        }
        KeyCode::Delete => {
            if let Some((input, cursor)) = app.create_mode.input_mut() {
                if *cursor < input.len() {
                    input.remove(*cursor);
                }
            }
        }
        KeyCode::Left => {
            if let Some((_input, cursor)) = app.create_mode.input_mut() {
                if *cursor > 0 {
                    *cursor -= 1;
                }
            }
        }
        KeyCode::Right => {
            if let Some((input, cursor)) = app.create_mode.input_mut() {
                if *cursor < input.len() {
                    *cursor += 1;
                }
            }
        }
        KeyCode::Home => {
            if let Some((_input, cursor)) = app.create_mode.input_mut() {
                *cursor = 0;
            }
        }
        KeyCode::End => {
            if let Some((input, cursor)) = app.create_mode.input_mut() {
                *cursor = input.len();
            }
        }
        _ => {}
    }
    AppAction::None
}

// ---------------------------------------------------------------------------
// Environment editor input
// ---------------------------------------------------------------------------

fn handle_env_input(app: &mut App, key: KeyEvent) -> AppAction {
    if app.editing {
        // Editing a cell in the env table
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                app.editing = false;
            }
            KeyCode::Tab => {
                // Toggle key ↔ value column
                app.env_edit_key = !app.env_edit_key;
                if let Some(row) = app.env_rows.get(app.env_selected) {
                    app.env_cursor = if app.env_edit_key { row.key.len() } else { row.value.len() };
                }
            }
            KeyCode::Char(c)
                if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT =>
            {
                if let Some(row) = app.env_rows.get_mut(app.env_selected) {
                    let (text, cursor) = if app.env_edit_key {
                        (&mut row.key, &mut app.env_cursor)
                    } else {
                        (&mut row.value, &mut app.env_cursor)
                    };
                    if *cursor > text.len() { *cursor = text.len(); }
                    text.insert(*cursor, c);
                    *cursor += c.len_utf8();
                }
            }
            KeyCode::Backspace => {
                if let Some(row) = app.env_rows.get_mut(app.env_selected) {
                    let (text, cursor) = if app.env_edit_key {
                        (&mut row.key, &mut app.env_cursor)
                    } else {
                        (&mut row.value, &mut app.env_cursor)
                    };
                    if *cursor > text.len() { *cursor = text.len(); }
                    if *cursor > 0 {
                        *cursor -= 1;
                        text.remove(*cursor);
                    }
                }
            }
            KeyCode::Delete => {
                if let Some(row) = app.env_rows.get_mut(app.env_selected) {
                    let (text, cursor) = if app.env_edit_key {
                        (&mut row.key, &mut app.env_cursor)
                    } else {
                        (&mut row.value, &mut app.env_cursor)
                    };
                    if *cursor > text.len() { *cursor = text.len(); }
                    if *cursor < text.len() { text.remove(*cursor); }
                }
            }
            KeyCode::Left => {
                if app.env_cursor > 0 { app.env_cursor -= 1; }
            }
            KeyCode::Right => {
                if let Some(row) = app.env_rows.get(app.env_selected) {
                    let len = if app.env_edit_key { row.key.len() } else { row.value.len() };
                    if app.env_cursor < len { app.env_cursor += 1; }
                }
            }
            KeyCode::Home => app.env_cursor = 0,
            KeyCode::End => {
                if let Some(row) = app.env_rows.get(app.env_selected) {
                    app.env_cursor = if app.env_edit_key { row.key.len() } else { row.value.len() };
                }
            }
            _ => {}
        }
        return AppAction::None;
    }

    // Navigation mode inside env editor
    match (key.code, key.modifiers) {
        (KeyCode::Esc, _) | (KeyCode::Char('e'), KeyModifiers::NONE) => {
            app.show_env_editor = false;
        }
        (KeyCode::Up, _) => {
            if app.env_selected > 0 { app.env_selected -= 1; }
        }
        (KeyCode::Down, _) => {
            if app.env_selected + 1 < app.env_rows.len() { app.env_selected += 1; }
        }
        (KeyCode::Enter, _) => {
            if !app.env_rows.is_empty() {
                app.env_edit_key = true;
                if let Some(row) = app.env_rows.get(app.env_selected) {
                    app.env_cursor = row.key.len();
                }
                app.editing = true;
            }
        }
        (KeyCode::Tab, _) => {
            // Switch to editing value column of selected row
            if !app.env_rows.is_empty() {
                app.env_edit_key = false;
                if let Some(row) = app.env_rows.get(app.env_selected) {
                    app.env_cursor = row.value.len();
                }
                app.editing = true;
            }
        }
        // Add new variable row
        (KeyCode::Char('o'), KeyModifiers::NONE) => {
            app.env_rows.push(EnvRow { key: String::new(), value: String::new() });
            app.env_selected = app.env_rows.len() - 1;
            app.env_edit_key = true;
            app.env_cursor = 0;
            app.editing = true;
        }
        // Delete selected row
        (KeyCode::Char('d'), KeyModifiers::NONE) => {
            if !app.env_rows.is_empty() {
                app.env_rows.remove(app.env_selected);
                if app.env_selected >= app.env_rows.len() && app.env_selected > 0 {
                    app.env_selected -= 1;
                }
            }
        }
        _ => {}
    }
    AppAction::None
}

// ---------------------------------------------------------------------------
// Change-root directory dialog input
// ---------------------------------------------------------------------------

fn handle_dir_input(app: &mut App, key: KeyEvent) -> AppAction {
    match (key.code, key.modifiers) {
        (KeyCode::Esc, _) | (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
            app.show_dir_input = false;
            app.dir_input = String::new();
            app.dir_input_cursor = 0;
            app.status_message = "Directory change cancelled".to_string();
        }
        (KeyCode::Enter, _) => {
            app.apply_dir_input();
        }
        (KeyCode::Tab, _) => {
            // Simple tab-completion: expand the longest existing prefix on disk
            let current = app.dir_input.clone();
            let path = std::path::Path::new(&current);
            let (parent, prefix) = if current.ends_with(std::path::MAIN_SEPARATOR)
                || current.ends_with('/')
            {
                (path, "")
            } else {
                (
                    path.parent().unwrap_or(path),
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(""),
                )
            };
            if let Ok(entries) = std::fs::read_dir(parent) {
                let mut matches: Vec<String> = entries
                    .flatten()
                    .filter(|e| e.path().is_dir())
                    .filter_map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        if name.starts_with(prefix) { Some(e.path().to_string_lossy().to_string()) }
                        else { None }
                    })
                    .collect();
                matches.sort();
                if let Some(first) = matches.first() {
                    let completed = first.clone();
                    app.dir_input_cursor = completed.len();
                    app.dir_input = completed;
                }
            }
        }
        (KeyCode::Backspace, _) => {
            if app.dir_input_cursor > 0 {
                let cursor = app.dir_input_cursor;
                let char_start = app.dir_input[..cursor]
                    .char_indices()
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                app.dir_input.remove(char_start);
                app.dir_input_cursor = char_start;
            }
        }
        (KeyCode::Delete, _) => {
            let cursor = app.dir_input_cursor;
            if cursor < app.dir_input.len() {
                app.dir_input.remove(cursor);
            }
        }
        (KeyCode::Left, _) => {
            if app.dir_input_cursor > 0 {
                let cursor = app.dir_input_cursor;
                app.dir_input_cursor = app.dir_input[..cursor]
                    .char_indices()
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
        }
        (KeyCode::Right, _) => {
            let cursor = app.dir_input_cursor;
            if cursor < app.dir_input.len() {
                if let Some((i, c)) = app.dir_input[cursor..].char_indices().next() {
                    app.dir_input_cursor = cursor + i + c.len_utf8();
                }
            }
        }
        (KeyCode::Home, _) => app.dir_input_cursor = 0,
        (KeyCode::End, _) => app.dir_input_cursor = app.dir_input.len(),
        (KeyCode::Char(c), m)
            if m == KeyModifiers::NONE || m == KeyModifiers::SHIFT =>
        {
            let cursor = app.dir_input_cursor;
            app.dir_input.insert(cursor, c);
            app.dir_input_cursor += c.len_utf8();
        }
        _ => {}
    }
    AppAction::None
}

// ---------------------------------------------------------------------------
// Delete confirmation dialog input
// ---------------------------------------------------------------------------

fn handle_confirm_delete(app: &mut App, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
            app.confirm_delete = ConfirmDelete::None;
            app.status_message = "Delete cancelled".to_string();
        }
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.execute_delete();
        }
        _ => {}
    }
    AppAction::None
}

// ---------------------------------------------------------------------------
// Text input (editing = true)
// ---------------------------------------------------------------------------

fn handle_text_input(app: &mut App, key: KeyEvent) -> AppAction {
    // Esc always exits editing
    if key.code == KeyCode::Esc {
        app.editing = false;
        return AppAction::None;
    }

    // Ctrl+Enter / Enter in URL sends request
    if key.code == KeyCode::Enter && app.editor_field == EditorField::Url {
        app.is_loading = true;
        app.status_message = "Sending request...".to_string();
        app.editing = false;
        return AppAction::SendRequest;
    }

    match app.editor_field {
        EditorField::Name => edit_name(app, key),
        EditorField::Url => edit_url(app, key),
        EditorField::Headers => edit_header(app, key),
        EditorField::Body => edit_body(app, key),
        EditorField::Method => {
            app.editing = false;
        }
    }
    AppAction::None
}

fn edit_name(app: &mut App, key: KeyEvent) {
    if key.code == KeyCode::Enter {
        app.editing = false;
        return;
    }
    let name = &mut app.current_request.name;
    // Clamp cursor defensively — prevents panic if cursor became stale
    if app.name_cursor > name.len() {
        app.name_cursor = name.len();
    }
    let cursor = &mut app.name_cursor;
    match key.code {
        KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT => {
            name.insert(*cursor, c);
            *cursor += c.len_utf8();
        }
        KeyCode::Backspace => {
            if *cursor > 0 {
                *cursor -= 1;
                name.remove(*cursor);
            }
        }
        KeyCode::Delete => {
            if *cursor < name.len() {
                name.remove(*cursor);
            }
        }
        KeyCode::Left => {
            if *cursor > 0 { *cursor -= 1; }
        }
        KeyCode::Right => {
            if *cursor < name.len() { *cursor += 1; }
        }
        KeyCode::Home => *cursor = 0,
        KeyCode::End => *cursor = name.len(),
        _ => {}
    }
}

fn edit_url(app: &mut App, key: KeyEvent) {
    let url = &mut app.current_request.url;
    if app.url_cursor > url.len() { app.url_cursor = url.len(); }
    let cursor = &mut app.url_cursor;
    match key.code {
        KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT => {
            url.insert(*cursor, c);
            *cursor += c.len_utf8();
        }
        KeyCode::Backspace => {
            if *cursor > 0 {
                *cursor -= 1;
                url.remove(*cursor);
            }
        }
        KeyCode::Delete => {
            if *cursor < url.len() {
                url.remove(*cursor);
            }
        }
        KeyCode::Left => {
            if *cursor > 0 { *cursor -= 1; }
        }
        KeyCode::Right => {
            if *cursor < url.len() { *cursor += 1; }
        }
        KeyCode::Home => *cursor = 0,
        KeyCode::End => *cursor = url.len(),
        KeyCode::Char('w') if key.modifiers == KeyModifiers::CONTROL => {
            while *cursor > 0 && url.chars().nth(*cursor - 1).map_or(false, |c| c == ' ') {
                *cursor -= 1; url.remove(*cursor);
            }
            while *cursor > 0 && url.chars().nth(*cursor - 1).map_or(false, |c| c != ' ') {
                *cursor -= 1; url.remove(*cursor);
            }
        }
        KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
            url.clear(); *cursor = 0;
        }
        // Tab while editing URL: exit editing (stay in editor; Tab = panel cycle only when not editing)
        KeyCode::Tab => {
            app.editing = false;
        }
        _ => {}
    }
}

fn edit_header(app: &mut App, key: KeyEvent) {
    let selected = app.header_selected;
    if app.header_rows.is_empty() || selected >= app.header_rows.len() {
        app.editing = false;
        return;
    }

    let is_key = app.header_edit_key;
    let cursor = &mut app.header_cursor;

    match key.code {
        KeyCode::Tab => {
            // Switch between key and value columns
            app.header_edit_key = !app.header_edit_key;
            app.header_cursor = if app.header_edit_key {
                app.header_rows[selected].key.len()
            } else {
                app.header_rows[selected].value.len()
            };
        }
        KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT => {
            let field = if is_key { &mut app.header_rows[selected].key } else { &mut app.header_rows[selected].value };
            field.insert(*cursor, c);
            *cursor += c.len_utf8();
        }
        KeyCode::Backspace => {
            let field = if is_key { &mut app.header_rows[selected].key } else { &mut app.header_rows[selected].value };
            if *cursor > 0 { *cursor -= 1; field.remove(*cursor); }
        }
        KeyCode::Delete => {
            let field = if is_key { &mut app.header_rows[selected].key } else { &mut app.header_rows[selected].value };
            if *cursor < field.len() { field.remove(*cursor); }
        }
        KeyCode::Left => { if *cursor > 0 { *cursor -= 1; } }
        KeyCode::Right => {
            let len = if is_key { app.header_rows[selected].key.len() } else { app.header_rows[selected].value.len() };
            if *cursor < len { *cursor += 1; }
        }
        KeyCode::Home => *cursor = 0,
        KeyCode::End => {
            *cursor = if is_key { app.header_rows[selected].key.len() } else { app.header_rows[selected].value.len() };
        }
        KeyCode::Enter | KeyCode::Down => {
            if selected + 1 < app.header_rows.len() {
                app.header_selected += 1;
                app.header_edit_key = true;
                app.header_cursor = 0;
            } else {
                app.editing = false;
            }
        }
        KeyCode::Up => {
            if selected > 0 {
                app.header_selected -= 1;
                app.header_edit_key = true;
                app.header_cursor = 0;
            }
        }
        KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
            let field = if is_key { &mut app.header_rows[selected].key } else { &mut app.header_rows[selected].value };
            field.clear(); *cursor = 0;
        }
        _ => {}
    }
}

fn edit_body(app: &mut App, key: KeyEvent) {
    let body = app.current_request.body.get_or_insert_with(String::new);
    let cursor = &mut app.body_cursor;
    *cursor = (*cursor).min(body.len());

    match key.code {
        KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT => {
            body.insert(*cursor, c);
            *cursor += c.len_utf8();
        }
        KeyCode::Enter => {
            body.insert(*cursor, '\n');
            *cursor += 1;
        }
        KeyCode::Backspace => {
            if *cursor > 0 {
                // step back one char boundary
                let prev = body[..*cursor]
                    .char_indices()
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                body.remove(prev);
                *cursor = prev;
            }
        }
        KeyCode::Delete => {
            if *cursor < body.len() { body.remove(*cursor); }
        }
        KeyCode::Left => {
            if *cursor > 0 {
                *cursor = body[..*cursor]
                    .char_indices()
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
        }
        KeyCode::Right => {
            if *cursor < body.len() {
                if let Some((_, c)) = body[*cursor..].char_indices().next() {
                    *cursor += c.len_utf8();
                }
            }
        }
        KeyCode::Home => {
            // Jump to start of current line
            let line_start = body[..*cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
            *cursor = line_start;
        }
        KeyCode::End => {
            // Jump to end of current line
            let line_end = body[*cursor..].find('\n').map(|i| *cursor + i).unwrap_or(body.len());
            *cursor = line_end;
        }
        KeyCode::Up => {
            // Move to same column on the previous line
            let col = *cursor - body[..*cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
            if let Some(prev_nl) = body[..*cursor].rfind('\n') {
                // prev_nl is end of the line before current; find its start
                let prev_line_start = body[..prev_nl].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let prev_line_len = prev_nl - prev_line_start;
                *cursor = prev_line_start + col.min(prev_line_len);
            }
            // already on first line — stay
        }
        KeyCode::Down => {
            // Move to same column on the next line
            let line_start = body[..*cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let col = *cursor - line_start;
            if let Some(next_nl) = body[*cursor..].find('\n') {
                let next_line_start = *cursor + next_nl + 1;
                let next_line_end = body[next_line_start..]
                    .find('\n')
                    .map(|i| next_line_start + i)
                    .unwrap_or(body.len());
                let next_line_len = next_line_end - next_line_start;
                *cursor = next_line_start + col.min(next_line_len);
            }
            // already on last line — stay
        }
        KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
            body.clear(); *cursor = 0;
        }
        KeyCode::Char('w') if key.modifiers == KeyModifiers::CONTROL => {
            // delete word backwards (stop at space or newline)
            while *cursor > 0 {
                let prev = body[..*cursor].char_indices().last().map(|(i, _)| i).unwrap_or(0);
                let ch = body[prev..].chars().next().unwrap_or(' ');
                if ch == ' ' || ch == '\n' { break; }
                body.remove(prev);
                *cursor = prev;
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Mouse event handler
// ---------------------------------------------------------------------------

pub fn handle_mouse(app: &mut App, mouse: MouseEvent) -> AppAction {
    // Ignore mouse input when dialogs are open
    if app.show_help || app.show_env_editor || app.show_dir_input
        || app.create_mode.is_active() || app.confirm_delete.is_active()
    {
        return AppAction::None;
    }

    let x = mouse.column;
    let y = mouse.row;

    match mouse.kind {
        // ── Scroll wheel ──────────────────────────────────────────────────
        MouseEventKind::ScrollDown => {
            if rect_contains(app.response_rect, x, y) {
                app.response_scroll = app.response_scroll.saturating_add(3);
            } else if rect_contains(app.request_rect, x, y) {
                if app.editing && app.editor_field == EditorField::Body {
                    app.body_scroll = app.body_scroll.saturating_add(3);
                }
            } else if rect_contains(app.collections_rect, x, y) {
                app.tree_selected = (app.tree_selected + 1)
                    .min(app.tree_items.len().saturating_sub(1));
            }
        }
        MouseEventKind::ScrollUp => {
            if rect_contains(app.response_rect, x, y) {
                app.response_scroll = app.response_scroll.saturating_sub(3);
            } else if rect_contains(app.request_rect, x, y) {
                if app.editing && app.editor_field == EditorField::Body {
                    app.body_scroll = app.body_scroll.saturating_sub(3);
                }
            } else if rect_contains(app.collections_rect, x, y) {
                app.tree_selected = app.tree_selected.saturating_sub(1);
            }
        }

        // ── Left click ────────────────────────────────────────────────────
        MouseEventKind::Down(MouseButton::Left) => {
            if rect_contains(app.collections_rect, x, y) {
                // Focus collections panel, stop editing
                if app.focus == FocusPanel::CollectionTree && !app.editing {
                    // Second click on same panel → select the item at cursor
                    if y >= app.tree_items_y {
                        let visible_height = app.collections_rect.height.saturating_sub(4);
                        let scroll_offset = if app.tree_selected >= visible_height as usize {
                            app.tree_selected - visible_height as usize + 1
                        } else {
                            0
                        };
                        let row = (y - app.tree_items_y) as usize;
                        let item_idx = scroll_offset + row;
                        if item_idx < app.tree_items.len() {
                            if app.tree_selected == item_idx {
                                app.tree_select_item();
                            } else {
                                app.tree_selected = item_idx;
                            }
                        }
                    }
                } else {
                    app.focus = FocusPanel::CollectionTree;
                    app.editing = false;
                    // Also select the clicked row
                    if y >= app.tree_items_y {
                        let visible_height = app.collections_rect.height.saturating_sub(4);
                        let scroll_offset = if app.tree_selected >= visible_height as usize {
                            app.tree_selected - visible_height as usize + 1
                        } else {
                            0
                        };
                        let row = (y - app.tree_items_y) as usize;
                        let item_idx = scroll_offset + row;
                        if item_idx < app.tree_items.len() {
                            app.tree_selected = item_idx;
                        }
                    }
                }
            } else if rect_contains(app.request_rect, x, y) {
                app.focus = FocusPanel::RequestEditor;
                if app.editing { app.editing = false; }
            } else if rect_contains(app.response_rect, x, y) {
                app.focus = FocusPanel::ResponseViewer;
            }
        }

        _ => {}
    }

    AppAction::None
}

fn rect_contains(rect: ratatui::layout::Rect, x: u16, y: u16) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

