use crate::app::{App, CreateMode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(f: &mut Frame, app: &App) {
    let (title, input, cursor, parent_path) = match &app.create_mode {
        CreateMode::None => return,
        CreateMode::Folder { input, cursor, parent_path } => {
            (" 📁 New Folder ", input.as_str(), *cursor, parent_path)
        }
        CreateMode::Collection { input, cursor, parent_path } => {
            (" 📄 New Collection (.yaml) ", input.as_str(), *cursor, parent_path)
        }
    };

    // Show relative path from collections root (falls back to full path)
    let rel_path = parent_path
        .strip_prefix(&app.collections_root)
        .ok()
        .and_then(|p| p.to_str())
        .filter(|s| !s.is_empty())
        .map(|s| format!("/{}", s.replace('\\', "/")))
        .unwrap_or_else(|| "/".to_string());

    let area = centered_rect(62, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // path label
            Constraint::Length(1), // input
            Constraint::Length(1), // hint
        ])
        .split(inner);

    // Show target directory
    let path_line = Line::from(vec![
        Span::styled("In: ", Style::default().fg(Color::DarkGray)),
        Span::styled(rel_path, Style::default().fg(Color::Cyan)),
    ]);
    f.render_widget(Paragraph::new(path_line), layout[0]);

    // Input line with cursor
    let cursor = cursor.min(input.len());
    let before = &input[..cursor];
    let after = &input[cursor..];
    let (cursor_char, rest) = if let Some(c) = after.chars().next() {
        let clen = c.len_utf8();
        (c.to_string(), &after[clen..])
    } else {
        (" ".to_string(), "")
    };

    let input_line = Line::from(vec![
        Span::raw(before),
        Span::styled(
            cursor_char,
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
        Span::raw(rest),
    ]);

    let hint_line = Line::from(Span::styled(
        "  Enter to confirm  ·  Esc to cancel",
        Style::default().fg(Color::DarkGray),
    ));

    f.render_widget(Paragraph::new(input_line), layout[1]);
    f.render_widget(Paragraph::new(hint_line), layout[2]);
}

/// Return a horizontally-centered rect of fixed width (max `width` cols), 5 rows tall
fn centered_rect(width: u16, area: Rect) -> Rect {
    let height = 5u16;
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}
