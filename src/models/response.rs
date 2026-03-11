use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct AppResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub elapsed_ms: u128,
}

impl AppResponse {
    pub fn status_color(&self) -> ratatui::style::Color {
        match self.status {
            200..=299 => ratatui::style::Color::Green,
            300..=399 => ratatui::style::Color::Yellow,
            400..=499 => ratatui::style::Color::Red,
            500..=599 => ratatui::style::Color::LightRed,
            _ => ratatui::style::Color::White,
        }
    }

    pub fn pretty_body(&self) -> String {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&self.body) {
            serde_json::to_string_pretty(&v).unwrap_or_else(|_| self.body.clone())
        } else {
            self.body.clone()
        }
    }
}
