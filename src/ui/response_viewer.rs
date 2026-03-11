use crate::app::{App, FocusPanel, ResponseTab};
use crate::ui::panel_block;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, Tabs},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == FocusPanel::ResponseViewer;
    let block = panel_block("Response", focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.is_loading {
        let loading = Paragraph::new("⏳ Sending request...")
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_widget(loading, inner);
        return;
    }

    let Some(response) = &app.response else {
        let hint = Paragraph::new(vec![
            Line::from(Span::styled(
                "No response yet.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "Press [r] or [:send] to send the request.",
                Style::default().fg(Color::DarkGray),
            )),
        ]);
        f.render_widget(hint, inner);
        return;
    };

    // Status line + tabs
    let top_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    render_status_line(f, response, top_chunks[0]);
    render_tabs(f, app, top_chunks[1]);
    render_tab_content(f, app, response, top_chunks[2]);
}

fn render_status_line(
    f: &mut Frame,
    response: &crate::models::AppResponse,
    area: Rect,
) {
    let status_style = Style::default()
        .fg(response.status_color())
        .add_modifier(Modifier::BOLD);

    let line = Line::from(vec![
        Span::styled(
            format!(" {} {} ", response.status, response.status_text),
            status_style,
        ),
        Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}ms", response.elapsed_ms),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} bytes", response.body.len()),
            Style::default().fg(Color::Gray),
        ),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let tab_titles = vec![" Body [1] ", " Headers [2] "];
    let selected_tab = match app.response_tab {
        ResponseTab::Body => 0,
        ResponseTab::Headers => 1,
    };
    let tabs = Tabs::new(tab_titles)
        .select(selected_tab)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        );
    f.render_widget(tabs, area);
}

fn render_tab_content(
    f: &mut Frame,
    app: &App,
    response: &crate::models::AppResponse,
    area: Rect,
) {
    match app.response_tab {
        ResponseTab::Body => render_body(f, app, response, area),
        ResponseTab::Headers => render_headers(f, response, area),
    }
}

fn render_body(
    f: &mut Frame,
    app: &App,
    response: &crate::models::AppResponse,
    area: Rect,
) {
    let body = response.pretty_body();
    let lines: Vec<Line> = body
        .lines()
        .map(|line| {
            // Basic JSON syntax coloring
            let colored = colorize_json_line(line);
            Line::from(colored)
        })
        .collect();

    let total_lines = lines.len() as u16;
    let scroll = app.response_scroll.min(total_lines.saturating_sub(1));

    let para = Paragraph::new(lines)
        .scroll((scroll, app.response_scroll_x))
        .block(Block::default().borders(Borders::NONE));

    // Split area: content | v-scrollbar, then content | h-scrollbar row
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    // Reserve bottom row for horizontal scrollbar hint
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(chunks[0]);

    f.render_widget(para, v_chunks[0]);

    // Vertical scrollbar
    if total_lines > v_chunks[0].height {
        let mut scrollbar_state = ScrollbarState::new(total_lines as usize)
            .position(scroll as usize);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            chunks[1],
            &mut scrollbar_state,
        );
    }

    // Horizontal scroll hint at bottom
    if app.response_scroll_x > 0 {
        let hint = format!("← → scroll  col:{}", app.response_scroll_x);
        f.render_widget(
            ratatui::widgets::Paragraph::new(hint)
                .style(Style::default().fg(Color::DarkGray)),
            v_chunks[1],
        );
    }
}

fn render_headers(
    f: &mut Frame,
    response: &crate::models::AppResponse,
    area: Rect,
) {
    let mut sorted_headers: Vec<(&String, &String)> = response.headers.iter().collect();
    sorted_headers.sort_by_key(|(k, _)| k.as_str());

    let rows: Vec<Row> = sorted_headers
        .iter()
        .map(|(k, v)| {
            Row::new(vec![
                Cell::from(k.as_str()).style(Style::default().fg(Color::Yellow)),
                Cell::from(v.as_str()).style(Style::default().fg(Color::White)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [Constraint::Percentage(35), Constraint::Percentage(65)],
    )
    .header(
        Row::new(vec![
            Cell::from("Header").style(
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
    .column_spacing(1);

    f.render_widget(table, area);
}

/// Naive JSON line colorizer: colors keys, strings, numbers, booleans, null
fn colorize_json_line(line: &str) -> Vec<Span<'static>> {
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];
    let mut spans = vec![Span::raw(indent.to_string())];

    // Try to detect key: "value" pattern
    if let Some(colon_pos) = trimmed.find(':') {
        let key_part = trimmed[..colon_pos].trim();
        let val_part = trimmed[colon_pos + 1..].trim();

        if key_part.starts_with('"') && key_part.ends_with('"') {
            spans.push(Span::styled(
                key_part.to_string(),
                Style::default().fg(Color::Cyan),
            ));
            spans.push(Span::styled(": ".to_string(), Style::default().fg(Color::DarkGray)));
            spans.push(colorize_value(val_part));
            return spans;
        }
    }

    // Not a key: value pair — just colorize the whole trimmed line
    spans.push(colorize_value(trimmed));
    spans
}

fn colorize_value(val: &str) -> Span<'static> {
    let val = val.trim_end_matches(',');
    if val.starts_with('"') {
        Span::styled(val.to_string(), Style::default().fg(Color::Green))
    } else if val == "true" || val == "false" {
        Span::styled(val.to_string(), Style::default().fg(Color::Yellow))
    } else if val == "null" {
        Span::styled(val.to_string(), Style::default().fg(Color::Red))
    } else if val.parse::<f64>().is_ok() {
        Span::styled(val.to_string(), Style::default().fg(Color::Magenta))
    } else {
        Span::styled(val.to_string(), Style::default().fg(Color::White))
    }
}
