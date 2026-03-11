use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(f: &mut Frame, app: &App) {
    let area = centered_rect(70, f.area());
    f.render_widget(Clear, area);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " Change Root Directory ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));

    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // Layout inside the outer block:
    //   row 0 — label          (1 line)
    //   row 1 — spacer         (1 line)
    //   row 2 — input box      (3 lines: border + text + border)
    //   row 3 — spacer         (1 line)
    //   row 4 — hint bar       (1 line)
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    // Label
    f.render_widget(
        Paragraph::new("Enter the path to your collections folder:"),
        rows[0],
    );

    // Input box with cursor
    let input = &app.dir_input;
    let cursor = app.dir_input_cursor.min(input.len());
    let before = &input[..cursor];
    let after = &input[cursor..];
    let cursor_char = after.chars().next().unwrap_or(' ');
    let after_cursor: String = after.chars().skip(1).collect();

    let input_line = Line::from(vec![
        Span::raw(before.to_string()),
        Span::styled(
            cursor_char.to_string(),
            Style::default().bg(Color::Cyan).fg(Color::Black),
        ),
        Span::raw(after_cursor),
    ]);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White));
    let input_inner = input_block.inner(rows[2]);
    f.render_widget(input_block, rows[2]);
    f.render_widget(Paragraph::new(input_line), input_inner);

    // Hint bar
    let hint = Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Green)),
        Span::raw(" confirm  ·  "),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" complete  ·  "),
        Span::styled("Esc", Style::default().fg(Color::Red)),
        Span::raw(" cancel  ·  current: "),
        Span::styled(
            app.collections_root.to_string_lossy().to_string(),
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        ),
    ]);
    f.render_widget(Paragraph::new(hint), rows[4]);
}

fn centered_rect(percent_x: u16, area: Rect) -> Rect {
    // Total popup height: 2(outer border) + 1(label) + 1(gap) + 3(input) + 1(gap) + 1(hint) = 9
    let popup_height: u16 = 9;
    let top_pad = area.height.saturating_sub(popup_height) / 2;

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_pad),
            Constraint::Length(popup_height),
            Constraint::Min(0),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
