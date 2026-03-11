use crate::app::{App, EnvRow};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};

pub fn render(f: &mut Frame, app: &App) {
    if !app.show_env_editor {
        return;
    }

    let area = centered_rect(75, 65, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .title(Span::styled(
            " 🌍 Environment Variables  [e / Esc] Close  [Ctrl+S] Save ",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Layout: hint bar (1) + table (rest)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    let hint = Line::from(vec![
        Span::styled("[o] Add  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[d] Delete  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Enter] Edit key  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Tab] Edit value  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[↑↓] Navigate  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Use {{VAR_NAME}} in URLs, headers, body", Style::default().fg(Color::Yellow)),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[0]);

    if app.env_rows.is_empty() {
        let msg = Paragraph::new("No variables defined. Press [o] to add one.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(msg, chunks[1]);
        return;
    }

    let rows: Vec<Row> = app
        .env_rows
        .iter()
        .enumerate()
        .map(|(i, row)| build_row(i, row, app))
        .collect();

    let table = Table::new(
        rows,
        [Constraint::Percentage(35), Constraint::Percentage(65)],
    )
    .header(
        Row::new(vec![
            Cell::from("Variable Name").style(
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Cell::from("Value").style(
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
        .bottom_margin(1),
    )
    .block(Block::default().borders(Borders::NONE))
    .column_spacing(2);

    f.render_widget(table, chunks[1]);
}

fn build_row<'a>(i: usize, row: &'a EnvRow, app: &App) -> Row<'a> {
    let is_selected = i == app.env_selected;
    let is_editing = is_selected && app.editing;

    let key_cell = if is_editing && app.env_edit_key {
        render_editing_cell(&row.key, app.env_cursor)
    } else {
        let style = if is_selected && !app.env_edit_key {
            Style::default().fg(Color::Black).bg(Color::Magenta)
        } else if is_selected {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::Yellow)
        };
        Cell::from(row.key.clone()).style(style)
    };

    let val_cell = if is_editing && !app.env_edit_key {
        render_editing_cell(&row.value, app.env_cursor)
    } else {
        let style = if is_selected && app.env_edit_key {
            Style::default().fg(Color::Black).bg(Color::Magenta)
        } else if is_selected {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };
        Cell::from(row.value.clone()).style(style)
    };

    Row::new(vec![key_cell, val_cell])
}

fn render_editing_cell(text: &str, cursor: usize) -> Cell<'static> {
    let cursor = cursor.min(text.len());
    let before = text[..cursor].to_string();
    let after = &text[cursor..];
    let cur_char: String = after
        .chars()
        .next()
        .map(|c| c.to_string())
        .unwrap_or_else(|| " ".to_string());
    let rest: String = if after.len() > cur_char.len() {
        after[cur_char.len()..].to_string()
    } else {
        String::new()
    };
    Cell::from(Line::from(vec![
        Span::raw(before),
        Span::styled(
            cur_char,
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
        Span::raw(rest),
    ]))
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
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
