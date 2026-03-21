//! Anthropic Messages API client — raw HTTP via `reqwest` (blocking).
//!
//! Uses SSE streaming so large responses never hit a request timeout.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::BufRead;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
pub const DEFAULT_MODEL: &str = "claude-opus-4-6";

// --------------------------------------------------------------------------
// Request / response types
// --------------------------------------------------------------------------

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    max_tokens: u32,
    stream: bool,
    system: &'a str,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'static str,
    content: &'a str,
}

// SSE event data shapes we care about.
#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamEvent {
    MessageStart { message: MessageStartPayload }, // id captured for future use
    ContentBlockDelta { delta: Delta },
    MessageDelta { delta: MessageDeltaPayload },
    #[serde(other)]
    Other,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct MessageStartPayload {
    id: String,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Delta {
    TextDelta { text: String },
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Debug)]
struct MessageDeltaPayload {
    stop_reason: Option<String>,
}

// --------------------------------------------------------------------------
// Public API
// --------------------------------------------------------------------------

/// Configuration for a single generation call.
pub struct LlmConfig<'a> {
    /// `ANTHROPIC_API_KEY` value (read from env by the caller).
    pub api_key: &'a str,
    /// Model ID — defaults to [`DEFAULT_MODEL`].
    pub model: &'a str,
    /// Maximum output tokens to request.
    pub max_tokens: u32,
    /// System prompt.
    pub system: &'a str,
    /// User message content.
    pub user_message: &'a str,
}

/// Call the Anthropic Messages API with streaming and return the full text.
///
/// Streams via SSE so the binary stays responsive even for large outputs.
pub fn call(cfg: &LlmConfig<'_>) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .context("Failed to build HTTP client")?;

    let body = Request {
        model: cfg.model,
        max_tokens: cfg.max_tokens,
        stream: true,
        system: cfg.system,
        messages: vec![Message {
            role: "user",
            content: cfg.user_message,
        }],
    };

    let response = client
        .post(API_URL)
        .header("x-api-key", cfg.api_key)
        .header("anthropic-version", API_VERSION)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .context("HTTP request to Anthropic API failed")?;

    let status = response.status();
    if !status.is_success() {
        let body_text = response
            .text()
            .unwrap_or_else(|_| "<unreadable body>".into());
        bail!("Anthropic API error {status}: {body_text}");
    }

    // Read the SSE stream line-by-line and accumulate text deltas.
    let mut accumulated = String::new();
    let reader = std::io::BufReader::new(response);

    for line in reader.lines() {
        let line = line.context("Error reading SSE stream")?;

        // SSE lines starting with "data: " carry JSON payloads.
        let Some(json_str) = line.strip_prefix("data: ") else {
            continue;
        };

        // The terminal event.
        if json_str.trim() == "[DONE]" {
            break;
        }

        // Parse each SSE event.
        let event: StreamEvent = match serde_json::from_str(json_str) {
            Ok(e) => e,
            Err(_) => continue, // skip unparseable lines gracefully
        };

        match event {
            StreamEvent::ContentBlockDelta {
                delta: Delta::TextDelta { text },
            } => {
                accumulated.push_str(&text);
            }
            StreamEvent::MessageDelta { delta } => {
                if delta.stop_reason.as_deref() == Some("max_tokens") {
                    log::warn!("Anthropic API response was truncated (max_tokens reached)");
                }
            }
            _ => {}
        }
    }

    Ok(accumulated)
}

// --------------------------------------------------------------------------
// Response parsing helpers
// --------------------------------------------------------------------------

/// Attempt to extract clean JSON from the LLM response.
///
/// The model is instructed to return bare JSON, but occasionally it may wrap
/// the response in markdown fences. This function handles both cases.
pub fn extract_json(raw: &str) -> &str {
    let trimmed = raw.trim();

    // Strip ```json ... ``` or ``` ... ``` fences if present.
    if let Some(inner) = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
    {
        if let Some(body) = inner.strip_suffix("```") {
            return body.trim();
        }
    }

    trimmed
}

/// Parse the raw LLM response into a `serde_json::Value`.
pub fn parse_response(raw: &str) -> Result<Value> {
    let json_str = extract_json(raw);
    serde_json::from_str(json_str)
        .with_context(|| format!("LLM response was not valid JSON:\n{raw}"))
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_strips_fences() {
        let input = "```json\n{\"hello\":1}\n```";
        assert_eq!(extract_json(input), "{\"hello\":1}");
    }

    #[test]
    fn extract_json_passthrough_bare() {
        let input = "{\"hello\":1}";
        assert_eq!(extract_json(input), input);
    }
}
