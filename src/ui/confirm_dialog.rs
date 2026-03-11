use crate::app::{App, ConfirmDelete};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(f: &mut Frame, app: &App) {
    let description = match &app.confirm_delete {
        ConfirmDelete::None => return,
        ConfirmDelete::Pending { description, .. } => description.clone(),
    };

    let area = centered_rect(64, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(Span::styled(
            " ⚠  Confirm Delete ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Three lines: warning text, blank, yes/no hint
    let msg_line = Line::from(vec![
        Span::styled("Delete ", Style::default().fg(Color::White)),
        Span::styled(&description, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled("?", Style::default().fg(Color::White)),
    ]);

    let warn_line = Line::from(Span::styled(
        "This cannot be undone!",
        Style::default().fg(Color::Red),
    ));

    let hint_line = Line::from(vec![
        Span::styled("  [y / Enter] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::styled("Yes, delete  ", Style::default().fg(Color::White)),
        Span::styled("  [n / Esc] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::styled("Cancel", Style::default().fg(Color::White)),
    ]);

    let lines = vec![msg_line, warn_line, Line::from(""), hint_line];
    f.render_widget(Paragraph::new(lines), inner);
}

/// Return a horizontally-centered rect of fixed width, 6 rows tall
fn centered_rect(width: u16, area: Rect) -> Rect {
    let height = 6u16;
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}
