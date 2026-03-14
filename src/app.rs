use crate::models::{AppResponse, Environment, HttpMethod, Request};
use crate::storage::yaml::{CollectionFile, TreeNode, TreeNodeKind, build_tree, load_collections};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Inline create dialog state
#[derive(Debug, Clone, Default)]
pub enum CreateMode {
    #[default]
    None,
    Folder { input: String, cursor: usize, parent_path: PathBuf },
    Collection { input: String, cursor: usize, parent_path: PathBuf },
}

impl CreateMode {
    pub fn is_active(&self) -> bool {
        !matches!(self, CreateMode::None)
    }

    pub fn input_mut(&mut self) -> Option<(&mut String, &mut usize)> {
        match self {
            CreateMode::Folder { input, cursor, .. } | CreateMode::Collection { input, cursor, .. } => {
                Some((input, cursor))
            }
            CreateMode::None => None,
        }
    }

    pub fn parent_path(&self) -> Option<&PathBuf> {
        match self {
            CreateMode::Folder { parent_path, .. } | CreateMode::Collection { parent_path, .. } => Some(parent_path),
            CreateMode::None => None,
        }
    }
}

/// Delete confirmation dialog state
#[derive(Debug, Clone, Default)]
pub enum ConfirmDelete {
    #[default]
    None,
    Pending { path: PathBuf, description: String },
}

impl ConfirmDelete {
    pub fn is_active(&self) -> bool {
        !matches!(self, ConfirmDelete::None)
    }
}

/// Which panel currently has focus
#[derive(Debug, Clone, PartialEq, Default)]
pub enum FocusPanel {
    #[default]
    CollectionTree,
    RequestEditor,
    ResponseViewer,
}

/// Which tab is active in the request editor
#[derive(Debug, Clone, PartialEq, Default)]
pub enum RequestTab {
    #[default]
    Headers,
    Body,
}

/// Which tab is active in the response viewer
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ResponseTab {
    #[default]
    Body,
    Headers,
}

/// Which field in the request editor has focus
#[derive(Debug, Clone, PartialEq, Default)]
pub enum EditorField {
    Name,
    #[default]
    Url,
    Method,
    Headers,
    Body,
}

#[derive(Debug, Clone, Default)]
pub struct HeaderRow {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Default)]
pub struct EnvRow {
    pub key: String,
    pub value: String,
}

pub struct App {
    // --- Focus & editing state ---
    pub focus: FocusPanel,
    /// true = a text field is actively receiving typed input
    pub editing: bool,
    pub should_quit: bool,

    // --- Panel rects (updated every render frame, used for mouse hit-testing) ---
    pub collections_rect: ratatui::layout::Rect,
    pub request_rect: ratatui::layout::Rect,
    pub response_rect: ratatui::layout::Rect,
    /// Absolute y of the first tree item row (after breadcrumb header)
    pub tree_items_y: u16,

    // --- Collections ---
    pub collections: Vec<CollectionFile>,
    pub tree_nodes: Vec<TreeNode>,

    // Which collection files are expanded (showing their requests as sub-items)
    pub collection_expanded: HashSet<usize>,

    // Flattened visible list of tree items for navigation
    pub tree_items: Vec<TreeItem>,
    pub tree_selected: usize,

    // --- Request Editor ---
    pub current_request: Request,
    pub current_collection_path: Option<std::path::PathBuf>,
    pub current_request_index: Option<usize>,
    pub request_tab: RequestTab,
    pub editor_field: EditorField,

    // Header editing state
    pub header_rows: Vec<HeaderRow>,
    pub header_selected: usize,
    pub header_edit_key: bool, // true = editing key column, false = editing value column

    // Cursor position for text inputs
    pub url_cursor: usize,
    pub name_cursor: usize,
    pub body_cursor: usize,
    pub body_scroll: u16,
    pub body_scroll_x: u16,
    pub header_cursor: usize,

    // --- Response ---
    pub response: Option<AppResponse>,
    pub response_tab: ResponseTab,
    pub response_scroll: u16,
    pub response_scroll_x: u16,
    pub is_loading: bool,

    // --- Response body text selection (content coordinates: line, col) ---
    pub response_sel_start: Option<(usize, usize)>,
    pub response_sel_end: Option<(usize, usize)>,
    pub response_is_selecting: bool,
    /// Rect of the body text area, stored each frame for mouse hit-testing
    pub response_body_rect: ratatui::layout::Rect,

    // --- Status message ---
    pub status_message: String,

    // --- Help overlay ---
    pub show_help: bool,

    // --- Create folder/file dialog ---
    pub create_mode: CreateMode,

    // --- Delete confirmation dialog ---
    pub confirm_delete: ConfirmDelete,

    // --- Environment variables ---
    pub env: Environment,
    pub show_env_editor: bool,
    /// Editable rows in the env editor (mirrors env.variables)
    pub env_rows: Vec<EnvRow>,
    pub env_selected: usize,
    pub env_edit_key: bool,  // true = editing key col, false = editing value col
    pub env_cursor: usize,

    // Root directory for collections (used for reloading and path resolution)
    pub collections_root: PathBuf,

    // --- Change-root directory input dialog ---
    pub show_dir_input: bool,
    pub dir_input: String,
    pub dir_input_cursor: usize,
}


/// A flattened tree item for rendering
#[derive(Debug, Clone)]
pub struct TreeItem {
    pub display: String,
    pub depth: usize,
    pub kind: TreeItemKind,
}

#[derive(Debug, Clone)]
pub enum TreeItemKind {
    Directory {
        path: std::path::PathBuf,
        expanded: bool,
    },
    File {
        collection_index: usize,
        expanded: bool,
    },
    Request {
        collection_index: usize,
        request_index: usize,
    },
}

impl App {
    pub fn new(
        collections: Vec<CollectionFile>,
        tree_nodes: Vec<TreeNode>,
        collections_root: PathBuf,
    ) -> Self {
        // Load environment variables
        let env_path = collections_root.join("env.yaml");
        let env = Environment::load(&env_path).unwrap_or_default();
        let env_rows: Vec<EnvRow> = {
            let mut rows: Vec<EnvRow> = env
                .variables
                .iter()
                .map(|(k, v)| EnvRow { key: k.clone(), value: v.clone() })
                .collect();
            rows.sort_by(|a, b| a.key.cmp(&b.key));
            rows
        };

        // All collections start collapsed; user expands with Enter
        let collection_expanded: HashSet<usize> = HashSet::new();
        // Use empty vec as placeholder; rebuild_tree_items() will fill correctly with root item
        let mut app = App {
            focus: FocusPanel::CollectionTree,
            editing: false,
            should_quit: false,
            collections_rect: ratatui::layout::Rect::default(),
            request_rect: ratatui::layout::Rect::default(),
            response_rect: ratatui::layout::Rect::default(),
            tree_items_y: 0,
            collection_expanded,
            collections,
            tree_nodes,
            tree_items: Vec::new(),
            tree_selected: 0,
            current_request: Request::new("New Request"),
            current_collection_path: None,
            current_request_index: None,
            request_tab: RequestTab::Headers,
            editor_field: EditorField::Url,
            header_rows: Vec::new(),
            header_selected: 0,
            header_edit_key: true,
            url_cursor: 0,
            name_cursor: 0,
            body_cursor: 0,
            body_scroll: 0,
            body_scroll_x: 0,
            header_cursor: 0,
            response: None,
            response_tab: ResponseTab::Body,
            response_scroll: 0,
            response_scroll_x: 0,
            is_loading: false,
            response_sel_start: None,
            response_sel_end: None,
            response_is_selecting: false,
            response_body_rect: ratatui::layout::Rect::default(),
            status_message: String::new(),
            show_help: false,
            create_mode: CreateMode::None,
            confirm_delete: ConfirmDelete::None,
            env,
            show_env_editor: false,
            env_rows,
            env_selected: 0,
            env_edit_key: true,
            env_cursor: 0,
            collections_root,
            show_dir_input: false,
            dir_input: String::new(),
            dir_input_cursor: 0,
        };
        app.rebuild_tree_items();
        // If there's a first request, load it
        if !app.collections.is_empty() && !app.collections[0].collection.requests.is_empty() {
            app.load_request(0, 0);
        }
        app
    }

    /// Load a request from collections[col_idx].requests[req_idx] into the editor
    pub fn load_request(&mut self, col_idx: usize, req_idx: usize) {
        if let Some(col_file) = self.collections.get(col_idx) {
            if let Some(req) = col_file.collection.requests.get(req_idx) {
                self.current_request = req.clone();
                self.current_collection_path = Some(col_file.path.clone());
                self.current_request_index = Some(req_idx);
                // Sync header rows
                self.header_rows = req
                    .headers
                    .iter()
                    .map(|(k, v)| HeaderRow {
                        key: k.clone(),
                        value: v.clone(),
                    })
                    .collect();
                self.url_cursor = self.current_request.url.len();
                self.name_cursor = self.current_request.name.len();
                self.body_cursor = self.current_request.body.as_deref().unwrap_or("").len();
                self.body_scroll = 0;
                self.body_scroll_x = 0;
                self.header_selected = 0;
                self.status_message = format!(
                    "Loaded: {}",
                    self.current_request.name
                );
            }
        }
    }

    /// Apply header_rows back into current_request.headers
    pub fn sync_headers_to_request(&mut self) {
        let mut map = HashMap::new();
        for row in &self.header_rows {
            if !row.key.is_empty() {
                map.insert(row.key.clone(), row.value.clone());
            }
        }
        self.current_request.headers = map;
    }

    /// Save current_request back to the appropriate collection file
    pub fn save_current_request(&mut self) -> anyhow::Result<()> {
        self.sync_headers_to_request();
        let path = self
            .current_collection_path
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No collection file associated"))?;
        let idx = self.current_request_index;

        if let Some(col_file) = self
            .collections
            .iter_mut()
            .find(|c| c.path == path)
        {
            if let Some(req_idx) = idx {
                if let Some(req) = col_file.collection.requests.get_mut(req_idx) {
                    *req = self.current_request.clone();
                } else {
                    col_file.collection.requests.push(self.current_request.clone());
                    self.current_request_index = Some(col_file.collection.requests.len() - 1);
                }
            } else {
                col_file.collection.requests.push(self.current_request.clone());
                self.current_request_index = Some(col_file.collection.requests.len() - 1);
            }
            col_file.save()?;
            self.status_message = format!("Saved: {}", self.current_request.name);
        }
        Ok(())
    }

    pub fn rebuild_tree_items(&mut self) {
        let mut items = vec![TreeItem {
            display: "/".to_string(),
            depth: 0,
            kind: TreeItemKind::Directory {
                path: self.collections_root.clone(),
                expanded: true,
            },
        }];
        items.extend(build_tree_items(
            &self.tree_nodes,
            1,
            &self.collection_expanded,
            &self.collections,
        ));
        self.tree_items = items;
    }

    /// Re-scan the collections root directory and refresh all state
    pub fn reload_collections(&mut self) {
        let root = self.collections_root.clone();
        self.collections = load_collections(&root);
        self.tree_nodes = build_tree(&root, &self.collections);
        self.collection_expanded.clear();
        self.rebuild_tree_items();
        // Clamp selection
        if self.tree_selected >= self.tree_items.len() && !self.tree_items.is_empty() {
            self.tree_selected = self.tree_items.len() - 1;
        }
    }

    /// Return a breadcrumb string showing the directory context of the current selection.
    /// Format: "/ subfolder / nested"  relative to collections_root.
    pub fn selected_dir_display(&self) -> String {
        let path = self.get_create_parent_path();
        let rel = path
            .strip_prefix(&self.collections_root)
            .ok()
            .and_then(|p| p.to_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.replace('\\', "/"))
            .unwrap_or_default();

        if rel.is_empty() {
            "/".to_string()
        } else {
            format!(
                "/ {}",
                rel.split('/').collect::<Vec<_>>().join(" / ")
            )
        }
    }

    /// Return the directory in which the next create operation should place its item.
    /// Uses the currently selected tree item to determine context.
    pub fn get_create_parent_path(&self) -> PathBuf {
        if let Some(item) = self.tree_items.get(self.tree_selected) {
            match &item.kind {
                TreeItemKind::Directory { path, .. } => path.clone(),
                TreeItemKind::File { collection_index, .. }
                | TreeItemKind::Request { collection_index, .. } => self
                    .collections
                    .get(*collection_index)
                    .and_then(|c| c.path.parent().map(|p| p.to_path_buf()))
                    .unwrap_or_else(|| self.collections_root.clone()),
            }
        } else {
            self.collections_root.clone()
        }
    }

    /// Return the parent of the current selection (for sibling creation with N).
    /// - Directory selected → its parent (clamped to collections_root)
    /// - File/Request selected → same parent as that file
    /// - Nothing selected → root
    pub fn get_sibling_parent_path(&self) -> PathBuf {
        if let Some(item) = self.tree_items.get(self.tree_selected) {
            match &item.kind {
                TreeItemKind::Directory { path, .. } => {
                    let parent = path
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| self.collections_root.clone());
                    // Never go above collections root
                    if parent.starts_with(&self.collections_root) {
                        parent
                    } else {
                        self.collections_root.clone()
                    }
                }
                TreeItemKind::File { collection_index, .. }
                | TreeItemKind::Request { collection_index, .. } => self
                    .collections
                    .get(*collection_index)
                    .and_then(|c| c.path.parent().map(|p| p.to_path_buf()))
                    .unwrap_or_else(|| self.collections_root.clone()),
            }
        } else {
            self.collections_root.clone()
        }
    }

    /// Commit the current create_mode: write to disk, reload tree, clear dialog
    pub fn confirm_create(&mut self) {
        // Use the path embedded in CreateMode (set at dialog open time)
        let parent = self
            .create_mode
            .parent_path()
            .cloned()
            .unwrap_or_else(|| self.collections_root.clone());
        // Make sure the parent exists
        let _ = std::fs::create_dir_all(&parent);

        match self.create_mode.clone() {
            CreateMode::Folder { input, .. } if !input.trim().is_empty() => {
                let name = input.trim().to_string();
                let new_path = parent.join(&name);
                match std::fs::create_dir_all(&new_path) {
                    Ok(_) => {
                        self.status_message = format!("Created folder: {name}");
                        self.reload_collections();
                    }
                    Err(e) => self.status_message = format!("Error: {e}"),
                }
            }
            CreateMode::Collection { input, .. } if !input.trim().is_empty() => {
                let raw = input.trim().to_string();
                let filename = if raw.ends_with(".yaml") || raw.ends_with(".yml") {
                    raw.clone()
                } else {
                    format!("{raw}.yaml")
                };
                let stem = raw
                    .trim_end_matches(".yaml")
                    .trim_end_matches(".yml")
                    .to_string();
                let new_path = parent.join(&filename);
                if new_path.exists() {
                    self.status_message = format!("File already exists: {filename}");
                } else {
                    let template =
                        format!("name: {stem}\nrequests: []\n");
                    match std::fs::write(&new_path, template) {
                        Ok(_) => {
                            self.status_message =
                                format!("Created collection: {filename}");
                            self.reload_collections();
                        }
                        Err(e) => self.status_message = format!("Error: {e}"),
                    }
                }
            }
            _ => {} // empty input — just cancel
        }
        self.create_mode = CreateMode::None;
    }

    pub fn tree_navigate_up(&mut self) {
        if self.tree_selected > 0 {
            self.tree_selected -= 1;
        }
    }

    pub fn tree_navigate_down(&mut self) {
        if self.tree_selected + 1 < self.tree_items.len() {
            self.tree_selected += 1;
        }
    }

    /// Add a new blank request to the collection currently selected (File or Request item).
    /// Opens the editor focused on the Name field so the user can immediately rename it.
    pub fn add_new_request_to_selected(&mut self) {
        let col_idx = if let Some(item) = self.tree_items.get(self.tree_selected) {
            match &item.kind {
                TreeItemKind::File { collection_index, .. } => Some(*collection_index),
                TreeItemKind::Request { collection_index, .. } => Some(*collection_index),
                TreeItemKind::Directory { .. } => None,
            }
        } else {
            None
        };

        if let Some(col_idx) = col_idx {
            if let Some(col_file) = self.collections.get_mut(col_idx) {
                let new_req = Request::new("New Request");
                col_file.collection.requests.push(new_req);
                let req_idx = col_file.collection.requests.len() - 1;
                // Capture the path before reload so we can re-locate the collection afterwards
                let saved_path = col_file.path.clone();
                match col_file.save() {
                    Ok(_) => {
                        // reload_collections() may reorder collections, so re-find by path
                        self.reload_collections();
                        let new_col_idx = self
                            .collections
                            .iter()
                            .position(|c| c.path == saved_path)
                            .unwrap_or(col_idx);
                        // Expand this collection so the new request is visible
                        self.collection_expanded.insert(new_col_idx);
                        self.rebuild_tree_items();
                        // Load the new request but stay in collections panel
                        self.load_request(new_col_idx, req_idx);
                        self.name_cursor = self.current_request.name.len();
                        self.editor_field = EditorField::Name;
                        self.editing = true;
                        // don't change focus — stay in collections
                    }
                    Err(e) => {
                        self.status_message = format!("Error saving: {e}");
                    }
                }
            }
        } else {
            self.status_message =
                "Select a collection file first, then press 'a' to add a request".to_string();
        }
    }

    /// Delete the currently selected request from the tree.
    pub fn delete_selected_request(&mut self) {
        if let Some(item) = self.tree_items.get(self.tree_selected).cloned() {
            if let TreeItemKind::Request { collection_index, request_index } = item.kind {
                if let Some(col_file) = self.collections.get_mut(collection_index) {
                    if request_index < col_file.collection.requests.len() {
                        let name = col_file.collection.requests[request_index].name.clone();
                        col_file.collection.requests.remove(request_index);
                        match col_file.save() {
                            Ok(_) => {
                                // Clear editor if it was showing this request
                                if self.current_collection_path.as_ref() == Some(&col_file.path)
                                    && self.current_request_index == Some(request_index)
                                {
                                    self.current_request = Request::new("New Request");
                                    self.current_collection_path = None;
                                    self.current_request_index = None;
                                    self.name_cursor = 0;
                                    self.url_cursor = 0;
                                    self.header_rows.clear();
                                }
                                self.status_message = format!("Deleted request: {name}");
                                self.reload_collections();
                            }
                            Err(e) => {
                                self.status_message = format!("Error saving: {e}");
                            }
                        }
                    }
                }
            } else {
                self.status_message = "Select a request item to delete".to_string();
            }
        }
    }

    pub fn tree_select_item(&mut self) {
        if let Some(item) = self.tree_items.get(self.tree_selected).cloned() {
            match item.kind {
                TreeItemKind::Directory { path, .. } => {
                    // Root item (path == collections_root) is always expanded; skip toggle
                    if path != self.collections_root {
                        toggle_dir_in_nodes(&mut self.tree_nodes, &path);
                        self.rebuild_tree_items();
                    }
                }
                TreeItemKind::File { collection_index, expanded } => {
                    // Toggle expansion to show/hide request sub-items
                    if expanded {
                        self.collection_expanded.remove(&collection_index);
                    } else {
                        self.collection_expanded.insert(collection_index);
                    }
                    self.rebuild_tree_items();
                }
                TreeItemKind::Request { collection_index, request_index } => {
                    self.load_request(collection_index, request_index);
                    self.editor_field = EditorField::Url;
                    // don't change focus — stay in collections panel
                }
            }
        }
    }

    /// Show the delete confirmation dialog for the currently selected tree item.
    pub fn prompt_delete_selected(&mut self) {
        if let Some(item) = self.tree_items.get(self.tree_selected).cloned() {
            match &item.kind {
                TreeItemKind::Directory { path, .. } => {
                    // Cannot delete the root
                    if path == &self.collections_root {
                        self.status_message = "Cannot delete the root directory".to_string();
                        return;
                    }
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("this folder")
                        .to_string();
                    self.confirm_delete = ConfirmDelete::Pending {
                        path: path.clone(),
                        description: format!("folder \"{}\" and ALL its contents", name),
                    };
                }
                TreeItemKind::File { collection_index, .. } => {
                    if let Some(col) = self.collections.get(*collection_index) {
                        let name = col.path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("this file")
                            .to_string();
                        self.confirm_delete = ConfirmDelete::Pending {
                            path: col.path.clone(),
                            description: format!("file \"{}\"", name),
                        };
                    }
                }
                TreeItemKind::Request { .. } => {
                    self.delete_selected_request();
                }
            }
        }
    }

    /// Execute the pending delete after user confirmation.
    pub fn execute_delete(&mut self) {
        if let ConfirmDelete::Pending { path, .. } = self.confirm_delete.clone() {
            // Clear editor if it referenced something inside the deleted path
            if let Some(cur_path) = &self.current_collection_path.clone() {
                if cur_path.starts_with(&path) {
                    self.current_request = Request::new("New Request");
                    self.current_collection_path = None;
                    self.current_request_index = None;
                    self.name_cursor = 0;
                    self.url_cursor = 0;
                    self.header_rows.clear();
                }
            }

            let result = if path.is_dir() {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };

            match result {
                Ok(_) => {
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("item")
                        .to_string();
                    self.status_message = format!("Deleted: {name}");
                    self.reload_collections();
                }
                Err(e) => {
                    self.status_message = format!("Delete failed: {e}");
                }
            }
        }
        self.confirm_delete = ConfirmDelete::None;
    }

    // -------------------------------------------------------------------------
    // Environment variable helpers
    // -------------------------------------------------------------------------

    pub fn env_file_path(&self) -> PathBuf {
        self.collections_root.join("env.yaml")
    }

    /// Sync env_rows → self.env.variables, then save to disk
    pub fn save_env(&mut self) {
        let mut map = std::collections::HashMap::new();
        for row in &self.env_rows {
            if !row.key.trim().is_empty() {
                map.insert(row.key.trim().to_string(), row.value.clone());
            }
        }
        self.env.variables = map;
        let path = self.env_file_path();
        match self.env.save(&path) {
            Ok(_) => self.status_message = "Environment saved".to_string(),
            Err(e) => self.status_message = format!("Error saving env: {e}"),
        }
    }

    /// Sync self.env.variables → env_rows (sorted) — used after load/external edit
    #[allow(dead_code)]
    pub fn sync_env_to_rows(&mut self) {
        self.env_rows = {
            let mut rows: Vec<EnvRow> = self
                .env
                .variables
                .iter()
                .map(|(k, v)| EnvRow { key: k.clone(), value: v.clone() })
                .collect();
            rows.sort_by(|a, b| a.key.cmp(&b.key));
            rows
        };
    }

    /// Return a copy of `current_request` with all `{{VAR}}` substituted.
    // -------------------------------------------------------------------------
    // Response text selection
    // -------------------------------------------------------------------------

    /// Return (normalized_start, normalized_end) in content (line, col) order,
    /// or None if there is no active selection.
    pub fn normalized_response_selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let start = self.response_sel_start?;
        let end = self.response_sel_end?;
        if start <= end { Some((start, end)) } else { Some((end, start)) }
    }

    /// Extract the currently selected text from the response body. Returns None
    /// if there is no selection or no response.
    pub fn selected_response_text(&self) -> Option<String> {
        let (start, end) = self.normalized_response_selection()?;
        let body = self.response.as_ref()?.pretty_body();
        let lines: Vec<&str> = body.lines().collect();

        if start.0 == end.0 {
            let line = lines.get(start.0)?;
            let chars: Vec<char> = line.chars().collect();
            let s = start.1.min(chars.len());
            let e = (end.1 + 1).min(chars.len());
            if s >= e { return None; }
            Some(chars[s..e].iter().collect())
        } else {
            let mut result = String::new();
            for line_idx in start.0..=end.0 {
                let line = lines.get(line_idx).copied().unwrap_or("");
                let chars: Vec<char> = line.chars().collect();
                if line_idx == start.0 {
                    let s = start.1.min(chars.len());
                    result.push_str(&chars[s..].iter().collect::<String>());
                    result.push('\n');
                } else if line_idx == end.0 {
                    let e = (end.1 + 1).min(chars.len());
                    result.push_str(&chars[..e].iter().collect::<String>());
                } else {
                    result.push_str(line);
                    result.push('\n');
                }
            }
            Some(result)
        }
    }

    /// Copy the selected response text to the system clipboard. Returns a status
    /// message describing success or failure.
    pub fn copy_response_selection(&mut self) -> String {
        match self.selected_response_text() {
            None => "No text selected".to_string(),
            Some(text) => {
                match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text.clone())) {
                    Ok(()) => format!("Copied {} chars", text.len()),
                    Err(e) => format!("Clipboard error: {e}"),
                }
            }
        }
    }

    pub fn resolved_request(&self) -> Request {
        let env = &self.env;
        Request {
            name: self.current_request.name.clone(),
            method: self.current_request.method.clone(),
            url: env.substitute(&self.current_request.url),
            headers: self
                .current_request
                .headers
                .iter()
                .map(|(k, v)| (env.substitute(k), env.substitute(v)))
                .collect(),
            body: self.current_request.body.as_ref().map(|b| env.substitute(b)),
        }
    }

    // -------------------------------------------------------------------------
    // Change-root directory dialog
    // -------------------------------------------------------------------------

    /// Open the "change root directory" dialog, pre-filled with the current root.
    pub fn open_dir_input(&mut self) {
        self.dir_input = self.collections_root.to_string_lossy().to_string();
        self.dir_input_cursor = self.dir_input.len();
        self.show_dir_input = true;
    }

    /// Apply the typed path: reload all collections, tree, and env from the new root.
    pub fn apply_dir_input(&mut self) {
        let raw = self.dir_input.trim().to_string();
        self.show_dir_input = false;
        self.dir_input = String::new();
        self.dir_input_cursor = 0;

        let new_root = PathBuf::from(&raw);
        if !new_root.is_dir() {
            self.status_message = format!("Not a directory: {raw}");
            return;
        }

        // Reload from new root
        self.collections_root = new_root.clone();
        let collections = load_collections(&new_root);
        let tree_nodes = build_tree(&new_root, &collections);
        self.collections = collections;
        self.tree_nodes = tree_nodes;

        // Reset selection state
        self.tree_selected = 0;
        self.collection_expanded.clear();
        self.current_collection_path = None;
        self.current_request_index = None;
        self.current_request = crate::models::Request::new("New Request");
        self.name_cursor = 0;
        self.url_cursor = 0;
        self.response = None;
        self.header_rows.clear();

        // Reload env
        let env_path = new_root.join("env.yaml");
        self.env = Environment::load(&env_path).unwrap_or_default();
        let mut rows: Vec<EnvRow> = self.env.variables.iter()
            .map(|(k, v)| EnvRow { key: k.clone(), value: v.clone() })
            .collect();
        rows.sort_by(|a, b| a.key.cmp(&b.key));
        self.env_rows = rows;
        self.env_selected = 0;

        self.rebuild_tree_items();
        self.status_message = format!("Root changed to: {raw}");
    }

    pub fn next_method(&mut self) {
        let methods = HttpMethod::all();
        let current = methods
            .iter()
            .position(|m| m.to_string() == self.current_request.method.to_string())
            .unwrap_or(0);
        self.current_request.method = methods[(current + 1) % methods.len()].clone();
    }

    pub fn prev_method(&mut self) {
        let methods = HttpMethod::all();
        let current = methods
            .iter()
            .position(|m| m.to_string() == self.current_request.method.to_string())
            .unwrap_or(0);
        self.current_request.method =
            methods[(current + methods.len() - 1) % methods.len()].clone();
    }
}

fn build_tree_items(
    nodes: &[TreeNode],
    depth: usize,
    col_expanded: &HashSet<usize>,
    collections: &[CollectionFile],
) -> Vec<TreeItem> {
    let mut items = Vec::new();
    for node in nodes {
        match &node.kind {
            TreeNodeKind::Directory { children, expanded } => {
                items.push(TreeItem {
                    display: node.name.clone(),
                    depth,
                    kind: TreeItemKind::Directory {
                        path: node.path.clone(),
                        expanded: *expanded,
                    },
                });
                if *expanded {
                    items.extend(build_tree_items(children, depth + 1, col_expanded, collections));
                }
            }
            TreeNodeKind::File { collection_index } => {
                let col_idx = *collection_index;
                let file_expanded = col_expanded.contains(&col_idx);
                items.push(TreeItem {
                    display: node.name.clone(),
                    depth,
                    kind: TreeItemKind::File {
                        collection_index: col_idx,
                        expanded: file_expanded,
                    },
                });
                if file_expanded {
                    if let Some(col) = collections.get(col_idx) {
                        for (req_idx, req) in col.collection.requests.iter().enumerate() {
                            items.push(TreeItem {
                                display: req.name.clone(),
                                depth: depth + 1,
                                kind: TreeItemKind::Request {
                                    collection_index: col_idx,
                                    request_index: req_idx,
                                },
                            });
                        }
                    }
                }
            }
        }
    }
    items
}

fn toggle_dir_in_nodes(nodes: &mut Vec<TreeNode>, path: &std::path::Path) {
    for node in nodes.iter_mut() {
        if node.path == path {
            node.toggle_expand();
            return;
        }
        if let TreeNodeKind::Directory { ref mut children, .. } = node.kind {
            toggle_dir_in_nodes(children, path);
        }
    }
}
