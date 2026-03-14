use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(f: &mut Frame) {
    let area = centered_rect(82, 92, f.area());

    // Clear the background behind the popup
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " Keyboard Shortcuts  [?] or any key to close ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split into two columns
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    f.render_widget(left_column(), cols[0]);
    f.render_widget(right_column(), cols[1]);
}

fn section(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("── {} ──────────────────────────", title),
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    ))
}

fn key(k: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:<18}", k), Style::default().fg(Color::Cyan)),
        Span::styled(desc.to_string(), Style::default().fg(Color::White)),
    ])
}

fn blank() -> Line<'static> {
    Line::from("")
}

fn left_column() -> Paragraph<'static> {
    let lines: Vec<Line> = vec![
        section("Global (any panel)"),
        key("Tab", "Next panel  Collections→Request→Response"),
        key("Shift+Tab", "Previous panel (reverse cycle)"),
        key("W", "Jump directly to Collections panel"),
        key("E", "Jump directly to Request Editor panel"),
        key("V", "Jump directly to Response Viewer panel"),
        key("r", "Send HTTP request  (works from any panel)"),
        key("F5", "Send HTTP request  (alternative)"),
        key("Ctrl+S", "Save request / env variables"),
        key("Ctrl+O", "Change root collections directory"),
        key("e", "Open / close environment editor"),
        key("q  /  Ctrl+C", "Quit"),
        key("?", "Open / close this help"),
        blank(),
        section("Collections Panel"),
        key("↑ / ↓", "Navigate tree items"),
        key("Enter", "Expand/collapse folder  or  open request"),
        key("Home / End", "Jump to first / last item"),
        key("o", "Add new request inside selected collection"),
        key("d", "Delete selected item (confirm for file/folder)"),
        key("c", "New .yaml collection file inside selected dir"),
        key("f", "New folder inside selected directory"),
        key("R  (Shift+R)", "Refresh tree from disk"),
        blank(),
        section("Request Editor — Navigation"),
        key("↑ / ↓", "Move focus: Name → URL → headers → body"),
        key("←  (from URL bar)", "Move focus to Method box"),
        key("→ / ↓  (method)", "Return focus to URL bar"),
        key("Enter", "Start editing the focused field"),
        key("Esc", "Stop editing, stay in field"),
        key("m / M", "Cycle HTTP method forward / backward"),
        key("1", "Switch request sub-tab → Headers"),
        key("2", "Switch request sub-tab → Body"),
        key("o", "Add new header row  (Headers tab)"),
        key("d", "Delete selected header row  (Headers tab)"),
        key("n", "Clear editor / start a new blank request"),
    ];
    Paragraph::new(lines).alignment(Alignment::Left)
}

fn right_column() -> Paragraph<'static> {
    let lines: Vec<Line> = vec![
        section("Request Editor — While Editing"),
        key("Type normally", "Insert text at cursor"),
        key("← / →", "Move cursor left / right (char-aware)"),
        key("Home / End", "Jump to start / end of line"),
        key("Backspace / Delete", "Delete char before / after cursor"),
        key("Ctrl+W", "Delete word backwards"),
        key("Ctrl+U", "Clear the entire field"),
        key("Enter  (URL field)", "Send HTTP request immediately"),
        key("Tab  (header field)", "Switch Key column ↔ Value column"),
        key("↑ / ↓  (body)", "Move cursor to previous / next line"),
        key("Esc", "Stop editing"),
        blank(),
        section("Response Panel"),
        key("↑ / ↓", "Scroll one line up / down"),
        key("PageUp / PageDown", "Scroll 10 lines up / down"),
        key("Ctrl+U / Ctrl+D", "Scroll 10 lines up / down"),
        key("← / → (or h / l)", "Scroll left / right (4 cols each)"),
        key("Home", "Scroll to top-left"),
        key("End", "Scroll to bottom"),
        key("1", "Switch response sub-tab → Body"),
        key("2", "Switch response sub-tab → Headers"),
        key("y", "Copy selected text to clipboard"),
        key("Esc", "Clear text selection"),
        blank(),
        section("Environment Editor  [e]"),
        key("↑ / ↓", "Navigate variable rows"),
        key("o", "Add a new variable row"),
        key("d", "Delete selected variable row"),
        key("Enter", "Edit the Key column of selected row"),
        key("Tab", "Edit the Value column of selected row"),
        key("Ctrl+S", "Save variables to collections/env.yaml"),
        key("e  /  Esc", "Close environment editor"),
        blank(),
        section("Environment Variables — Usage"),
        key("{{BASE_URL}}", "Reference a variable in URL or body"),
        key("{{TOKEN}}", "Use in headers: Bearer {{TOKEN}}"),
    ];
    Paragraph::new(lines).alignment(Alignment::Left)
}

/// Return a centered rect of (percent_x × percent_y) within the given area
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
