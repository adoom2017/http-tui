use crate::app::{App, FocusPanel, TreeItemKind};
use crate::ui::panel_block;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == FocusPanel::CollectionTree;
    let block = panel_block("Collections", focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split inner into: path bar (1 line) + divider (1 line) + tree items
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    // ── path breadcrumb ──
    let dir_path = app.selected_dir_display();
    let path_line = Line::from(vec![
        Span::styled(" 📂 ", Style::default().fg(Color::Yellow)),
        Span::styled(dir_path, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Paragraph::new(path_line), chunks[0]);

    // ── thin divider ──
    let divider = Paragraph::new(Line::from(Span::styled(
        "─".repeat(inner.width as usize),
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(divider, chunks[1]);

    let tree_area = chunks[2];
    // Record where tree items start (for mouse click hit-testing)
    app.tree_items_y = tree_area.y;

    if app.tree_items.is_empty() {
        let msg = Paragraph::new("No collections found.\nPress [n] to create a .yaml\nor [N] to create a folder.")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(msg, tree_area);
        return;
    }

    let visible_height = tree_area.height as usize;
    let selected = app.tree_selected;

    // Compute scroll offset to keep selected item visible
    let scroll_offset = if selected >= visible_height {
        selected - visible_height + 1
    } else {
        0
    };

    let lines: Vec<Line> = app
        .tree_items
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(idx, item)| {
            let indent = "  ".repeat(item.depth);
            let prefix = match &item.kind {
                TreeItemKind::Directory { expanded, .. } => {
                    if *expanded { "▼ " } else { "▶ " }
                }
                TreeItemKind::File { expanded, .. } => {
                    if *expanded { "▼ " } else { "▶ " }
                }
                TreeItemKind::Request { .. } => "  ",
            };
            let icon = match &item.kind {
                TreeItemKind::Directory { .. } => "📁 ",
                TreeItemKind::File { .. } => "📄 ",
                TreeItemKind::Request { .. } => "• ",
            };

            let display = format!("{}{}{}{}", indent, prefix, icon, item.display);

            if idx == selected {
                Line::from(Span::styled(
                    display,
                    Style::default()
                        .fg(Color::Black)
                        .bg(if app.focus == FocusPanel::CollectionTree {
                            Color::Cyan
                        } else {
                            Color::Gray
                        })
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                let color = match &item.kind {
                    TreeItemKind::Directory { .. } => Color::Blue,
                    TreeItemKind::File { .. } => Color::Green,
                    TreeItemKind::Request { .. } => Color::White,
                };
                Line::from(Span::styled(display, Style::default().fg(color)))
            }
        })
        .collect();

    let list = Paragraph::new(lines);
    f.render_widget(list, tree_area);
}
