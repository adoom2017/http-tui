use crate::app::{App, FocusPanel, ResponseTab};
use crate::ui::panel_block;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, Tabs},
};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
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

    let Some(response) = app.response.clone() else {
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

    render_status_line(f, &response, top_chunks[0]);
    render_tabs(f, app, top_chunks[1]);
    render_tab_content(f, app, &response, top_chunks[2]);
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
    app: &mut App,
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
    app: &mut App,
    response: &crate::models::AppResponse,
    area: Rect,
) {
    let body = response.pretty_body();
    let sel = app.normalized_response_selection();

    let lines: Vec<Line> = body
        .lines()
        .enumerate()
        .map(|(line_idx, line)| {
            let colored = colorize_json_line(line);
            let spans = apply_selection_highlight(colored, line_idx, sel);
            Line::from(spans)
        })
        .collect();

    let total_lines = lines.len() as u16;
    let scroll = app.response_scroll.min(total_lines.saturating_sub(1));

    let para = Paragraph::new(lines)
        .scroll((scroll, app.response_scroll_x))
        .block(Block::default().borders(Borders::NONE));

    // Split area: content | v-scrollbar
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    // Reserve bottom row for horizontal scrollbar hint or selection hint
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(chunks[0]);

    // Store the body text rect for mouse hit-testing and selection coordinate mapping
    app.response_body_rect = v_chunks[0];

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

    // Bottom hint bar: show selection size, scroll position, or nothing
    let hint_text = if sel.is_some() {
        let char_count = app.selected_response_text().map(|t| t.len()).unwrap_or(0);
        format!("  {} chars selected — [y] copy", char_count)
    } else if app.response_scroll_x > 0 {
        format!("← → scroll  col:{}", app.response_scroll_x)
    } else {
        String::new()
    };
    if !hint_text.is_empty() {
        f.render_widget(
            Paragraph::new(hint_text).style(Style::default().fg(Color::DarkGray)),
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

/// Overlay selection highlighting on a pre-colored line.
///
/// `line_idx` is the 0-based content line index.
/// `sel` is the normalized (start, end) selection in (line, col) content coordinates.
///
/// Returns spans with a blue background applied to characters in the selected range.
fn apply_selection_highlight(
    spans: Vec<Span<'static>>,
    line_idx: usize,
    sel: Option<((usize, usize), (usize, usize))>,
) -> Vec<Span<'static>> {
    let (start, end) = match sel {
        None => return spans,
        Some(s) => s,
    };

    // Determine which character columns on this line are selected
    if line_idx < start.0 || line_idx > end.0 {
        return spans;
    }
    let col_start = if line_idx == start.0 { start.1 } else { 0 };
    let col_end = if line_idx == end.0 { end.1 } else { usize::MAX };

    const SEL_BG: Color = Color::Rgb(60, 90, 140);
    let mut result = Vec::with_capacity(spans.len() * 2);
    let mut pos = 0usize;

    for span in spans {
        let text = span.content.as_ref();
        let style = span.style;
        let chars: Vec<char> = text.chars().collect();
        let span_len = chars.len();
        let span_end = pos + span_len;

        if span_end <= col_start || pos > col_end {
            // Entirely outside selection
            result.push(Span::styled(text.to_string(), style));
        } else {
            // May overlap — split into [before] [selected] [after]
            let sel_local_start = col_start.saturating_sub(pos).min(span_len);
            let sel_local_end = col_end.saturating_add(1).saturating_sub(pos).min(span_len);

            if sel_local_start > 0 {
                let before: String = chars[..sel_local_start].iter().collect();
                result.push(Span::styled(before, style));
            }
            if sel_local_end > sel_local_start {
                let selected: String = chars[sel_local_start..sel_local_end].iter().collect();
                result.push(Span::styled(selected, style.bg(SEL_BG)));
            }
            if sel_local_end < span_len {
                let after: String = chars[sel_local_end..].iter().collect();
                result.push(Span::styled(after, style));
            }
        }
        pos += span_len;
    }
    result
}
