mod app;
mod events;
mod http;
mod models;
mod storage;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use events::handler::{AppAction, handle_key, handle_mouse};
use models::AppResponse;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::{io, path::PathBuf, time::Duration};
use storage::yaml::{build_tree, load_collections};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    // Determine collections root: first CLI arg, or ./collections by default
    let collections_dir = {
        let mut args = std::env::args().skip(1);
        if let Some(arg) = args.next() {
            let p = PathBuf::from(arg);
            if p.is_absolute() { p } else { std::env::current_dir()?.join(p) }
        } else {
            let mut p = std::env::current_dir()?;
            p.push("collections");
            p
        }
    };

    // Load collections from disk
    let collections = load_collections(&collections_dir);
    let tree_nodes = build_tree(&collections_dir, &collections);

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(collections, tree_nodes, collections_dir.clone());
    app.status_message = if collections_dir.exists() {
        format!("Collections loaded from: {}", collections_dir.display())
    } else {
        format!(
            "Collections dir not found: {}  — create it and add .yaml files",
            collections_dir.display()
        )
    };

    // Channel for async HTTP responses
    let (tx, mut rx) = mpsc::unbounded_channel::<Result<AppResponse, String>>();

    // Main event loop
    let result = run_app(&mut terminal, &mut app, &tx, &mut rx).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    tx: &mpsc::UnboundedSender<Result<AppResponse, String>>,
    rx: &mut mpsc::UnboundedReceiver<Result<AppResponse, String>>,
) -> Result<()> {
    loop {
        // Draw UI
        terminal.draw(|f| ui::render(f, app))?;

        // Check for async HTTP response (non-blocking)
        if let Ok(resp_result) = rx.try_recv() {
            app.is_loading = false;
            match resp_result {
                Ok(response) => {
                    app.status_message = format!(
                        "{} {} — {}ms",
                        response.status, response.status_text, response.elapsed_ms
                    );
                    app.response = Some(response);
                    app.response_scroll = 0;
                    app.response_scroll_x = 0;
                    app.response_sel_start = None;
                    app.response_sel_end = None;
                    // don't auto-jump to response panel — user stays where they are
                }
                Err(err) => {
                    app.status_message = format!("Request error: {err}");
                }
            }
        }

        // Poll for events (16ms ≈ 60fps)
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => {
                    // Only handle Press events — ignore Release and Repeat to prevent double-firing
                    if key.kind == KeyEventKind::Press {
                        let action = handle_key(app, key);
                        match action {
                            AppAction::Quit => { app.should_quit = true; }
                            AppAction::SendRequest => {
                                let request = app.resolved_request();
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    let result = http::send_request(&request)
                                        .await
                                        .map_err(|e| e.to_string());
                                    let _ = tx_clone.send(result);
                                });
                            }
                            AppAction::None => {}
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    let action = handle_mouse(app, mouse);
                    if action == AppAction::Quit { app.should_quit = true; }
                }
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
