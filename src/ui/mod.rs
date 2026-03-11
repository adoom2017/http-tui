pub mod collection_tree;
pub mod confirm_dialog;
pub mod create_dialog;
pub mod dir_input_dialog;
pub mod env_editor;
pub mod help;
pub mod request_editor;
pub mod response_viewer;

use crate::app::{App, FocusPanel};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn render(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Title bar (1 line) + main area + status bar (1 line)
    let top_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(size);

    render_title_bar(f, app, top_chunks[0]);

    // Main area: left panel + right panels
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(0)])
        .split(top_chunks[1]);

    collection_tree::render(f, app, main_chunks[0]);
    app.collections_rect = main_chunks[0];

    // Right side: request editor (top) + response viewer (bottom)
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(main_chunks[1]);

    request_editor::render(f, app, right_chunks[0]);
    app.request_rect = right_chunks[0];
    response_viewer::render(f, app, right_chunks[1]);
    app.response_rect = right_chunks[1];

    render_status_bar(f, app, top_chunks[2]);

    // Help overlay rendered last so it appears on top
    if app.show_help {
        help::render(f);
    }

    // Env editor overlay
    if app.show_env_editor {
        env_editor::render(f, app);
    }

    // Confirm delete dialog
    if app.confirm_delete.is_active() {
        confirm_dialog::render(f, app);
    }

    // Dir input dialog rendered on top of everything
    if app.show_dir_input {
        dir_input_dialog::render(f, app);
        return;
    }

    // Create dialog rendered on top of everything
    if app.create_mode.is_active() {
        create_dialog::render(f, app);
    }
}

fn render_title_bar(f: &mut Frame, app: &App, area: Rect) {
    let focused_name = match app.focus {
        FocusPanel::CollectionTree => "Collections",
        FocusPanel::RequestEditor => "Request",
        FocusPanel::ResponseViewer => "Response",
    };
    let title = Line::from(vec![
        Span::styled(" http-tui ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  {}  ", focused_name), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("│  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Tab] Cycle  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[W] Collections  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[E] Request  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[V] Response  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[r] Send  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Ctrl+S] Save  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[q] Quit  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[?] Help", Style::default().fg(Color::Yellow)),
    ]);
    let bar = Paragraph::new(title);
    f.render_widget(bar, area);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let edit_indicator = if app.editing {
        Span::styled(" EDITING ", Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" NAVIGATE ", Style::default().fg(Color::Black).bg(Color::Blue).add_modifier(Modifier::BOLD))
    };

    let status = if app.status_message.is_empty() {
        app.current_request.url.clone()
    } else {
        app.status_message.clone()
    };

    let spans = Line::from(vec![
        edit_indicator,
        Span::raw(" "),
        Span::styled(status, Style::default().fg(Color::White)),
    ]);
    let bar = Paragraph::new(spans).style(Style::default().bg(Color::DarkGray));
    f.render_widget(bar, area);
}

/// Helper: build a block with a title and highlight if focused
pub fn panel_block(title: &str, focused: bool) -> Block<'_> {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            format!(" {} ", title),
            if focused {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            },
        ))
}
