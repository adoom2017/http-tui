use crate::models::{AppResponse, HttpMethod, Request};
use anyhow::{Context, Result};
use std::time::Instant;

pub async fn send_request(request: &Request) -> Result<AppResponse> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .context("Failed to create HTTP client")?;

    let method = match request.method {
        HttpMethod::Get => reqwest::Method::GET,
        HttpMethod::Post => reqwest::Method::POST,
        HttpMethod::Put => reqwest::Method::PUT,
        HttpMethod::Patch => reqwest::Method::PATCH,
        HttpMethod::Delete => reqwest::Method::DELETE,
        HttpMethod::Head => reqwest::Method::HEAD,
        HttpMethod::Options => reqwest::Method::OPTIONS,
    };

    let mut builder = client.request(method, &request.url);

    for (key, value) in &request.headers {
        builder = builder.header(key, value);
    }

    if let Some(body) = &request.body {
        builder = builder.body(body.clone());
    }

    let start = Instant::now();
    let response = builder.send().await.context("Request failed")?;
    let elapsed_ms = start.elapsed().as_millis();

    let status = response.status();
    let status_code = status.as_u16();
    let status_text = status.canonical_reason().unwrap_or("Unknown").to_string();

    let headers: std::collections::HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| {
            (
                k.to_string(),
                v.to_str().unwrap_or("").to_string(),
            )
        })
        .collect();

    let body = response.text().await.unwrap_or_default();

    Ok(AppResponse {
        status: status_code,
        status_text,
        headers,
        body,
        elapsed_ms,
    })
}
