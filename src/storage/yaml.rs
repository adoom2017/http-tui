use crate::models::Collection;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Represents a YAML file on disk paired with its parsed Collection.
#[derive(Debug, Clone)]
pub struct CollectionFile {
    pub path: PathBuf,
    pub collection: Collection,
}

impl CollectionFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;
        let collection: Collection = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML: {}", path.display()))?;
        Ok(CollectionFile { path, collection })
    }

    pub fn save(&self) -> Result<()> {
        let content = serde_yaml::to_string(&self.collection)
            .context("Failed to serialize collection to YAML")?;
        std::fs::write(&self.path, content)
            .with_context(|| format!("Failed to write file: {}", self.path.display()))?;
        Ok(())
    }
}

/// Walk a root directory and load all .yaml / .yml files as collections.
pub fn load_collections(root: impl AsRef<Path>) -> Vec<CollectionFile> {
    let root = root.as_ref();
    if !root.exists() {
        return Vec::new();
    }

    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let path = entry.path();
            // Skip env.yaml — it is not a collection file
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if file_name == "env.yaml" || file_name == "env.yml" {
                return false;
            }
            path.is_file()
                && path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == "yaml" || e == "yml")
                    .unwrap_or(false)
        })
        .filter_map(|entry| CollectionFile::load(entry.path()).ok())
        .collect()
}

/// Build a tree structure representing the directory hierarchy.
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub path: PathBuf,
    pub kind: TreeNodeKind,
}

#[derive(Debug, Clone)]
pub enum TreeNodeKind {
    Directory { children: Vec<TreeNode>, expanded: bool },
    File { collection_index: usize },
}

impl TreeNode {
    pub fn toggle_expand(&mut self) {
        if let TreeNodeKind::Directory { ref mut expanded, .. } = self.kind {
            *expanded = !*expanded;
        }
    }
}

/// Build a tree of TreeNodes from a list of CollectionFiles under root.
pub fn build_tree(root: impl AsRef<Path>, collections: &[CollectionFile]) -> Vec<TreeNode> {
    let root = root.as_ref();
    build_tree_recursive(root, root, collections)
}

fn build_tree_recursive(
    root: &Path,
    dir: &Path,
    collections: &[CollectionFile],
) -> Vec<TreeNode> {
    let mut nodes: Vec<TreeNode> = Vec::new();

    // Collect direct children
    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return nodes,
    };

    let mut entries: Vec<std::fs::DirEntry> = read_dir
        .filter_map(|e| e.ok())
        .collect();

    // Sort: directories first, then files, alphabetically
    entries.sort_by(|a, b| {
        let a_is_dir = a.path().is_dir();
        let b_is_dir = b.path().is_dir();
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            let children = build_tree_recursive(root, &path, collections);
            // Always include directories, even if empty, so freshly created folders are visible
            nodes.push(TreeNode {
                name,
                path: path.clone(),
                kind: TreeNodeKind::Directory {
                    children,
                    expanded: true,
                },
            });
        } else if path.extension().and_then(|e| e.to_str()).map(|e| e == "yaml" || e == "yml").unwrap_or(false) {
            if let Some(idx) = collections.iter().position(|c| c.path == path) {
                nodes.push(TreeNode {
                    name: path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&name)
                        .to_string(),
                    path,
                    kind: TreeNodeKind::File { collection_index: idx },
                });
            }
        }
    }

    nodes
}
