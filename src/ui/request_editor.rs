use crate::app::{App, EditorField, FocusPanel, RequestTab};
use crate::models::HttpMethod;
use crate::ui::panel_block;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs},
};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == FocusPanel::RequestEditor;
    let block = panel_block("Request", focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Layout: name row | method+URL row | tabs | content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // name
            Constraint::Length(3), // method + URL
            Constraint::Length(1), // tabs
            Constraint::Min(0),    // content
        ])
        .split(inner);

    render_name(f, app, chunks[0]);
    render_method_url(f, app, chunks[1]);
    render_tabs(f, app, chunks[2]);
    render_tab_content(f, app, chunks[3]);
}

fn render_name(f: &mut Frame, app: &App, area: Rect) {
    let name_focused = app.focus == FocusPanel::RequestEditor
        && app.editor_field == EditorField::Name;
    let name_editing = name_focused && app.editing;

    let border_style = if name_editing {
        Style::default().fg(Color::Green)
    } else if name_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if name_focused && !name_editing {
        Span::styled(" Name [Enter] ", Style::default().fg(Color::Gray))
    } else {
        Span::styled(" Name ", Style::default().fg(Color::DarkGray))
    };

    let name_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    let name = &app.current_request.name;
    let name_display = if name_editing {
        let cursor = app.name_cursor.min(name.len());
        let before = &name[..cursor];
        let after = &name[cursor..];
        let cursor_char = after.chars().next().map(|c| c.to_string()).unwrap_or_else(|| " ".to_string());
        let rest = if after.len() > cursor_char.len() { &after[cursor_char.len()..] } else { "" };
        Line::from(vec![
            Span::raw(before),
            Span::styled(cursor_char, Style::default().fg(Color::Black).bg(Color::Green)),
            Span::raw(rest),
        ])
    } else {
        Line::from(Span::styled(
            name.as_str(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))
    };

    f.render_widget(
        Paragraph::new(name_display).block(name_block),
        area,
    );
}

fn render_method_url(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(10), Constraint::Min(0), Constraint::Length(12)])
        .split(area);

    // Method selector
    let method_focused = app.focus == FocusPanel::RequestEditor
        && !app.editing
        && app.editor_field == EditorField::Method;
    let method_style = if method_focused {
        Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        method_color(&app.current_request.method)
    };
    let method_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if method_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        })
        .title(if method_focused {
            Span::styled(" ←→ ", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        });
    let method_text = Paragraph::new(app.current_request.method.to_string())
        .style(method_style)
        .block(method_block);
    f.render_widget(method_text, chunks[0]);

    // URL input
    let url_focused = app.focus == FocusPanel::RequestEditor
        && app.editor_field == EditorField::Url;
    let url_editing = url_focused && app.editing;
    let url_border_style = if url_editing {
        Style::default().fg(Color::Green)
    } else if url_focused {
        Style::default().fg(Color::Yellow)  // focused but not editing
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let url_block = Block::default()
        .borders(Borders::ALL)
        .border_style(url_border_style)
        .title(if url_focused && !url_editing {
            Span::styled(" URL [Enter] ", Style::default().fg(Color::Gray))
        } else {
            Span::styled(" URL ", Style::default().fg(Color::DarkGray))
        });

    let url = &app.current_request.url;
    let url_display = if url_editing {
        let cursor = app.url_cursor.min(url.len());
        let (before, after) = url.split_at(cursor);
        let cursor_char = after.chars().next().map(|c| c.to_string()).unwrap_or_else(|| " ".to_string());
        let rest = if after.len() > cursor_char.len() { &after[cursor_char.len()..] } else { "" };
        Line::from(vec![
            Span::raw(before),
            Span::styled(cursor_char, Style::default().fg(Color::Black).bg(Color::Green)),
            Span::raw(rest),
        ])
    } else {
        // Highlight {{VAR}} tokens in magenta
        render_url_with_vars(url)
    };

    // If URL contains variables, show resolved value as a subtitle in the block title area
    let resolved = app.env.substitute(url);
    let url_block = if !url_editing && url.contains("{{") && resolved != *url {
        Block::default()
            .borders(Borders::ALL)
            .border_style(url_border_style)
            .title(Span::styled(" URL [Enter] ", Style::default().fg(Color::Gray)))
            .title_bottom(Span::styled(
                format!(" → {} ", resolved),
                Style::default().fg(Color::DarkGray),
            ))
    } else {
        url_block
    };

    let url_para = Paragraph::new(url_display)
        .style(Style::default().fg(Color::White))
        .block(url_block);
    f.render_widget(url_para, chunks[1]);

    // Send button hint
    let send_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let send_btn = Paragraph::new(" [r] Send ")
        .style(send_style)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Green)));
    f.render_widget(send_btn, chunks[2]);
}

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let tab_titles = vec![" Headers [1] ", " Body [2] "];
    let selected_tab = match app.request_tab {
        RequestTab::Headers => 0,
        RequestTab::Body => 1,
    };
    // Highlight yellow when tab area is focused but not editing
    let tab_focused = app.focus == FocusPanel::RequestEditor
        && matches!(app.editor_field, EditorField::Headers | EditorField::Body)
        && !app.editing;
    let tabs = Tabs::new(tab_titles)
        .select(selected_tab)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(if tab_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        });
    f.render_widget(tabs, area);
}

fn render_tab_content(f: &mut Frame, app: &mut App, area: Rect) {
    match app.request_tab {
        RequestTab::Headers => render_headers(f, app, area),
        RequestTab::Body => render_body(f, app, area),
    }
}

fn render_headers(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == FocusPanel::RequestEditor
        && app.editor_field == EditorField::Headers;

    let header_hint = Line::from(vec![
        Span::styled("[o] Add  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[d] Delete  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Enter] Edit  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Tab] Key↔Value", Style::default().fg(Color::DarkGray)),
    ]);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    f.render_widget(Paragraph::new(header_hint), chunks[0]);

    if app.header_rows.is_empty() {
        let msg = Paragraph::new("No headers. Press [o] to add a header.")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(msg, chunks[1]);
        return;
    }

    let rows: Vec<Row> = app
        .header_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_selected = i == app.header_selected && focused;
            let is_editing = is_selected && app.editing && app.editor_field == EditorField::Headers;

            let key_cell = if is_editing && app.header_edit_key {
                let key = &row.key;
                let cursor = app.header_cursor.min(key.len());
                let before = key[..cursor].to_string();
                let after = &key[cursor..];
                let cur_char: String = after.chars().next().map(|c| c.to_string()).unwrap_or_else(|| " ".to_string());
                let rest: String = if after.len() > cur_char.len() { after[cur_char.len()..].to_string() } else { String::new() };
                Cell::from(Line::from(vec![
                    Span::raw(before),
                    Span::styled(cur_char, Style::default().fg(Color::Black).bg(Color::Green)),
                    Span::raw(rest),
                ]))
            } else {
                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Yellow)
                };
                Cell::from(row.key.clone()).style(style)
            };

            let val_cell = if is_editing && !app.header_edit_key {
                let val = &row.value;
                let cursor = app.header_cursor.min(val.len());
                let before = val[..cursor].to_string();
                let after = &val[cursor..];
                let cur_char: String = after.chars().next().map(|c| c.to_string()).unwrap_or_else(|| " ".to_string());
                let rest: String = if after.len() > cur_char.len() { after[cur_char.len()..].to_string() } else { String::new() };
                Cell::from(Line::from(vec![
                    Span::raw(before),
                    Span::styled(cur_char, Style::default().fg(Color::Black).bg(Color::Green)),
                    Span::raw(rest),
                ]))
            } else {
                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };
                Cell::from(row.value.clone()).style(style)
            };

            Row::new(vec![key_cell, val_cell])
        })
        .collect();

    let table = Table::new(
        rows,
        [Constraint::Percentage(40), Constraint::Percentage(60)],
    )
    .header(
        Row::new(vec![
            Cell::from("Key").style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
            Cell::from("Value").style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
        ])
        .bottom_margin(1),
    )
    .block(Block::default().borders(Borders::NONE))
    .column_spacing(1);

    f.render_widget(table, chunks[1]);
}

fn render_body(f: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == FocusPanel::RequestEditor
        && app.editor_field == EditorField::Body
        && app.editing;

    let border_style = if focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let title = if focused {
        Span::styled(" Body  [Esc] Stop editing ", Style::default().fg(Color::Green))
    } else {
        Span::styled(" Body  [Enter] to edit ", Style::default().fg(Color::DarkGray))
    };
    let body_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    let body_text = app.current_request.body.as_deref().unwrap_or("").to_string();
    let cursor = app.body_cursor.min(body_text.len());

    // Compute cursor line and column (char-based column for display)
    let text_before_cursor = &body_text[..cursor];
    let cursor_line = text_before_cursor.chars().filter(|&c| c == '\n').count();
    let line_start = text_before_cursor.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let cursor_col = text_before_cursor[line_start..].chars().count();

    // Build lines with cursor highlight
    let all_lines: Vec<Line> = {
        let full = body_text.clone();
        let mut lines: Vec<Line> = Vec::new();
        let mut byte_pos: usize = 0;
        for raw_line in full.split('\n') {
            let line_start_b = byte_pos;
            let line_end_b = line_start_b + raw_line.len();
            if focused && cursor >= line_start_b && cursor <= line_end_b {
                let local = cursor - line_start_b;
                let lb = &raw_line[..local.min(raw_line.len())];
                let la = &raw_line[local.min(raw_line.len())..];
                let cur_ch = la.chars().next().unwrap_or(' ');
                let la_rest: String = la.chars().skip(1).collect();
                lines.push(Line::from(vec![
                    Span::raw(lb.to_string()),
                    Span::styled(cur_ch.to_string(), Style::default().bg(Color::Green).fg(Color::Black)),
                    Span::raw(la_rest),
                ]));
            } else {
                lines.push(Line::from(raw_line.to_string()));
            }
            byte_pos = line_end_b + 1;
        }
        lines
    };

    let total_lines = all_lines.len() as u16;
    let max_col: u16 = body_text
        .lines()
        .map(|l| l.chars().count() as u16)
        .max()
        .unwrap_or(0);

    // Inner area (inside block border): split off 1-col right scrollbar and 1-row bottom scrollbar
    let inner = body_block.inner(area);
    f.render_widget(body_block, area);

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(h_chunks[0]);

    let view_h = v_chunks[0].height;
    let view_w = v_chunks[0].width;

    // Auto-scroll vertically to keep cursor in view
    if focused {
        if cursor_line as u16 >= app.body_scroll + view_h {
            app.body_scroll = (cursor_line as u16).saturating_sub(view_h - 1);
        } else if (cursor_line as u16) < app.body_scroll {
            app.body_scroll = cursor_line as u16;
        }
        // Auto-scroll horizontally
        if cursor_col as u16 >= app.body_scroll_x + view_w {
            app.body_scroll_x = (cursor_col as u16).saturating_sub(view_w - 1);
        } else if (cursor_col as u16) < app.body_scroll_x {
            app.body_scroll_x = cursor_col as u16;
        }
    }

    let scroll_y = app.body_scroll.min(total_lines.saturating_sub(1));
    let scroll_x = app.body_scroll_x;

    // Render text
    let para = Paragraph::new(all_lines.clone())
        .scroll((scroll_y, scroll_x));
    f.render_widget(para, v_chunks[0]);

    // Vertical scrollbar
    if total_lines > view_h {
        let mut state = ratatui::widgets::ScrollbarState::new(total_lines as usize)
            .position(scroll_y as usize);
        f.render_stateful_widget(
            ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight),
            h_chunks[1],
            &mut state,
        );
    }

    // Horizontal scrollbar
    if max_col > view_w {
        let mut state = ratatui::widgets::ScrollbarState::new(max_col as usize)
            .position(scroll_x as usize);
        f.render_stateful_widget(
            ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::HorizontalBottom),
            v_chunks[1],
            &mut state,
        );
    }
}

fn method_color(method: &HttpMethod) -> Style {
    match method {
        HttpMethod::Get => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        HttpMethod::Post => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        HttpMethod::Put => Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
        HttpMethod::Patch => Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        HttpMethod::Delete => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        HttpMethod::Head => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        HttpMethod::Options => Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD),
    }
}

/// Parse a URL string and return a Line with `{{VAR}}` tokens highlighted in magenta.
fn render_url_with_vars(url: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut remaining = url;
    loop {
        if let Some(start) = remaining.find("{{") {
            if let Some(end) = remaining[start..].find("}}") {
                let end = start + end + 2;
                if start > 0 {
                    spans.push(Span::raw(remaining[..start].to_string()));
                }
                spans.push(Span::styled(
                    remaining[start..end].to_string(),
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                ));
                remaining = &remaining[end..];
            } else {
                break;
            }
        } else {
            break;
        }
    }
    if !remaining.is_empty() {
        spans.push(Span::raw(remaining.to_string()));
    }
    if spans.is_empty() {
        Line::from(url.to_string())
    } else {
        Line::from(spans)
    }
}
