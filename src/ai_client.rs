//! AI synthesis client
//!
//! Sends a raw transcript to the configured provider endpoint and returns
//! polished Markdown. Request building and response parsing are pure
//! functions so they can be unit tested without network access.
//!
//! Adding a new endpoint shape: add an `EndpointKind` variant in config.rs,
//! then extend `build_request_body` and `parse_response` here.

use crate::config::{AiProviderConfig, EndpointKind};
use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Build the JSON request body for a synthesis call.
/// `prompt` is the user-editable system prompt from config,
/// `transcript` is the raw Whisper output.
pub fn build_request_body(
    kind: EndpointKind,
    prompt: &str,
    transcript: &str,
    model: &str,
) -> Value {
    match kind {
        EndpointKind::OpenAiCompatible => json!({
            "model": model,
            "messages": [
                {"role": "system", "content": prompt},
                {"role": "user", "content": transcript}
            ],
            "stream": false
        }),
        EndpointKind::Anthropic => json!({
            "model": model,
            "system": prompt,
            "messages": [
                {"role": "user", "content": transcript}
            ],
            "max_tokens": 4096
        }),
    }
}

/// Extract the assistant's text from a provider response body.
pub fn parse_response(kind: EndpointKind, body: &Value) -> Result<String> {
    match kind {
        EndpointKind::OpenAiCompatible => body["choices"][0]["message"]["content"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| {
                anyhow!(
                    "unexpected response shape from OpenAI-compatible endpoint: {}",
                    summarize_error(body)
                )
            }),
        EndpointKind::Anthropic => body["content"][0]["text"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| {
                anyhow!(
                    "unexpected response shape from Anthropic endpoint: {}",
                    summarize_error(body)
                )
            }),
    }
}

fn summarize_error(body: &Value) -> String {
    let msg = body["error"]["message"]
        .as_str()
        .or_else(|| body["error"].as_str())
        .unwrap_or("unknown error");
    msg.chars().take(200).collect()
}

/// Send the synthesis request to the provider. Blocking — must be called
/// from a background thread only.
pub fn synthesize(provider: &AiProviderConfig, prompt: &str, transcript: &str) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .context("failed to build HTTP client")?;

    let url = match provider.kind {
        EndpointKind::OpenAiCompatible => {
            format!(
                "{}/chat/completions",
                provider.endpoint.trim_end_matches('/')
            )
        }
        EndpointKind::Anthropic => {
            format!("{}/messages", provider.endpoint.trim_end_matches('/'))
        }
    };

    let mut request = client.post(&url).json(&build_request_body(
        provider.kind,
        prompt,
        transcript,
        &provider.model,
    ));

    match provider.kind {
        EndpointKind::OpenAiCompatible => {
            if !provider.api_key.is_empty() {
                request = request.bearer_auth(&provider.api_key);
            }
        }
        EndpointKind::Anthropic => {
            request = request
                .header("x-api-key", &provider.api_key)
                .header("anthropic-version", "2023-06-01");
        }
    }

    let response = request
        .send()
        .context(format!("failed to reach AI endpoint at {}", url))?;

    let status = response.status();
    let body: Value = response.json().context(format!(
        "AI endpoint returned invalid JSON (HTTP {})",
        status
    ))?;

    if !status.is_success() {
        return Err(anyhow!(
            "AI endpoint error (HTTP {}): {}",
            status,
            summarize_error(&body)
        ));
    }

    parse_response(provider.kind, &body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_build_request_openai_compatible() {
        let body = build_request_body(
            EndpointKind::OpenAiCompatible,
            "sys prompt",
            "raw transcript text",
            "llama3.2",
        );
        assert_eq!(body["model"], "llama3.2");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "sys prompt");
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(body["messages"][1]["content"], "raw transcript text");
        assert_eq!(body["stream"], false);
    }

    #[test]
    fn test_build_request_anthropic() {
        let body = build_request_body(
            EndpointKind::Anthropic,
            "sys prompt",
            "raw transcript text",
            "claude-sonnet-4",
        );
        assert_eq!(body["model"], "claude-sonnet-4");
        assert_eq!(body["system"], "sys prompt");
        assert_eq!(body["messages"][0]["content"], "raw transcript text");
        assert_eq!(body["max_tokens"], 4096);
        // Anthropic puts the system prompt at top level, not in messages
        assert!(body["messages"][0]["system"].is_null());
    }

    #[test]
    fn test_parse_response_openai_compatible() {
        let body: Value = serde_json::from_str(
            "{\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":\"## Done\\n- thing\"}}]}",
        )
        .unwrap();
        let result = parse_response(EndpointKind::OpenAiCompatible, &body).unwrap();
        assert_eq!(result, "## Done\n- thing");
    }

    #[test]
    fn test_parse_response_anthropic() {
        let body: Value = serde_json::from_str(
            "{\"content\":[{\"type\":\"text\",\"text\":\"## Accomplishments\\n- shipped\"}]}",
        )
        .unwrap();
        let result = parse_response(EndpointKind::Anthropic, &body).unwrap();
        assert_eq!(result, "## Accomplishments\n- shipped");
    }

    #[test]
    fn test_parse_response_error_shapes() {
        let openai_bad: Value = serde_json::from_str(r#"{"choices":[]}"#).unwrap();
        assert!(parse_response(EndpointKind::OpenAiCompatible, &openai_bad).is_err());

        let anthropic_bad: Value = serde_json::from_str(r#"{"content":[]}"#).unwrap();
        assert!(parse_response(EndpointKind::Anthropic, &anthropic_bad).is_err());

        let api_error: Value = serde_json::from_str(
            r#"{"error":{"message":"model not found","type":"invalid_request_error"}}"#,
        )
        .unwrap();
        let err = parse_response(EndpointKind::OpenAiCompatible, &api_error)
            .unwrap_err()
            .to_string();
        assert!(err.contains("model not found"));
    }

    #[test]
    fn test_summarize_error_truncates() {
        let long: Value = serde_json::from_str(&format!(
            "{{\"error\":{{\"message\":\"{}\"}}}}",
            "x".repeat(500)
        ))
        .unwrap();
        let summary = summarize_error(&long);
        assert_eq!(summary.len(), 200);
    }
}
