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
// Generous: slow local models (9B on modest GPUs) can take minutes on long transcripts
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

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

/// Result of a resolved models fetch: ids plus the endpoint base that worked.
type ResolvedModels = (Vec<String>, String);

/// GET a models listing, trying the as-configured endpoint first and
/// falling back to `<base>/v1` when the first response is a 404 (the
/// signature of a bare-host base URL). Returns the models and the
/// endpoint that worked.
pub fn fetch_models_resolved(provider: &AiProviderConfig) -> Result<ResolvedModels> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(Duration::from_secs(30))
        .build()
        .context("failed to build HTTP client")?;

    let auth = |req: reqwest::blocking::RequestBuilder| match provider.kind {
        EndpointKind::OpenAiCompatible => {
            if provider.api_key.is_empty() {
                req
            } else {
                req.bearer_auth(&provider.api_key)
            }
        }
        EndpointKind::Anthropic => req
            .header("x-api-key", &provider.api_key)
            .header("anthropic-version", "2023-06-01"),
    };

    // Candidate endpoints in order: as-given, then /v1-suffixed
    let trimmed = provider.endpoint.trim_end_matches('/').to_string();
    let mut candidates: Vec<String> = vec![trimmed.clone()];
    if !trimmed.ends_with("/v1") {
        candidates.push(format!("{}/v1", trimmed));
    }

    let mut last_err: Option<anyhow::Error> = None;
    for candidate in &candidates {
        let url = format!("{}/models", candidate);
        let request = auth(client.get(&url));
        let response = match request.send() {
            Ok(r) => r,
            Err(e) => {
                last_err = Some(anyhow!(e).context(format!("failed to reach {}", url)));
                continue;
            }
        };
        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            last_err = Some(anyhow!("model list error (HTTP 404) at {}", url));
            continue;
        }
        let body: Value = match response.json() {
            Ok(b) => b,
            Err(e) => {
                last_err = Some(anyhow!(e).context(format!("invalid JSON from {}", url)));
                continue;
            }
        };
        if !status.is_success() {
            last_err = Some(anyhow!(
                "model list error (HTTP {}): {}",
                status,
                summarize_error(&body)
            ));
            continue;
        }
        return Ok((parse_models_response(&body), candidate.clone()));
    }
    Err(last_err.unwrap_or_else(|| anyhow!("no endpoint candidates tried")))
}

/// Extract model ids from a `/models` listing. OpenAI-compatible and
/// Anthropic both return {"data": [{"id": ...}, ...]}.
pub fn parse_models_response(body: &Value) -> Vec<String> {
    body["data"]
        .as_array()
        .map(|models| {
            models
                .iter()
                .filter_map(|m| m["id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Query the provider's model list. Blocking — background thread only.
pub fn fetch_models(provider: &AiProviderConfig) -> Result<Vec<String>> {
    fetch_models_resolved(provider).map(|(models, _)| models)
}

/// Send the synthesis request to the provider. Blocking — must be called
/// from a background thread only.
pub fn synthesize(provider: &AiProviderConfig, prompt: &str, transcript: &str) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .context("failed to build HTTP client")?;

    let action = match provider.kind {
        EndpointKind::OpenAiCompatible => "chat/completions",
        EndpointKind::Anthropic => "messages",
    };
    let base = provider.endpoint.trim_end_matches('/').to_string();

    // Candidate bases: as-given, then /v1-suffixed for bare hosts (404 fallback)
    let mut candidates: Vec<String> = vec![base.clone()];
    if provider.kind == EndpointKind::OpenAiCompatible && !base.ends_with("/v1") {
        candidates.push(format!("{}/v1", base));
    }

    let mut last_err: Option<anyhow::Error> = None;
    for candidate in &candidates {
        let url = format!("{}/{}", candidate, action);
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

        let response = match request.send() {
            Ok(r) => r,
            Err(e) => {
                last_err = Some(anyhow!(e).context(format!("failed to reach {}", url)));
                continue;
            }
        };
        let status = response.status();
        // A 404 means wrong API root — try the next candidate
        if status == reqwest::StatusCode::NOT_FOUND {
            last_err = Some(anyhow!("synthesis error (HTTP 404) at {}", url));
            continue;
        }
        let body: Value = match response.json() {
            Ok(b) => b,
            Err(e) => {
                last_err = Some(anyhow!(e).context(format!(
                    "AI endpoint returned invalid JSON (HTTP {})",
                    status
                )));
                continue;
            }
        };
        if !status.is_success() {
            return Err(anyhow!(
                "AI endpoint error (HTTP {}): {}",
                status,
                summarize_error(&body)
            ));
        }
        return parse_response(provider.kind, &body);
    }
    Err(last_err.unwrap_or_else(|| anyhow!("no endpoint candidates tried")))
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

    #[test]
    fn test_parse_models_response_openai_shape() {
        let body: Value = serde_json::from_str(
            r#"{"object":"list","data":[{"id":"llama3.2","object":"model"},{"id":"gpt-4o","object":"model"}]}"#,
        )
        .unwrap();
        let models = parse_models_response(&body);
        assert_eq!(models, vec!["llama3.2", "gpt-4o"]);
    }

    #[test]
    fn test_parse_models_response_empty_or_malformed() {
        assert!(parse_models_response(&serde_json::json!({})).is_empty());
        assert!(parse_models_response(&serde_json::json!({"data": []})).is_empty());
        assert!(parse_models_response(&serde_json::json!({"data": [{"nope": 1}]})).is_empty());
    }

    #[test]
    fn test_parse_models_response_skips_missing_ids() {
        let body: Value =
            serde_json::from_str(r#"{"data":[{"id":"a"},{"no_id":true},{"id":"b"}]}"#).unwrap();
        assert_eq!(parse_models_response(&body), vec!["a", "b"]);
    }

    #[test]
    fn test_endpoint_candidate_generation() {
        // Bare host should get a /v1 fallback candidate; /v1 base should not
        let base = "https://ollama.example.ts.net";
        let trimmed = base.trim_end_matches('/');
        let mut candidates: Vec<String> = vec![trimmed.to_string()];
        if !trimmed.ends_with("/v1") {
            candidates.push(format!("{}/v1", trimmed));
        }
        assert_eq!(
            candidates,
            vec![
                "https://ollama.example.ts.net".to_string(),
                "https://ollama.example.ts.net/v1".to_string()
            ]
        );

        // Explicit /v1 base: no extra candidate
        let base2 = "https://api.openai.com/v1/";
        let trimmed2 = base2.trim_end_matches('/');
        let mut candidates2: Vec<String> = vec![trimmed2.to_string()];
        if !trimmed2.ends_with("/v1") {
            candidates2.push(format!("{}/v1", trimmed2));
        }
        assert_eq!(candidates2, vec!["https://api.openai.com/v1".to_string()]);
    }

    #[test]
    fn test_fetch_models_resolved_missing_model_file_error_shape() {
        // Provider with unreachable endpoint should error with context
        let provider = AiProviderConfig {
            name: "bad".to_string(),
            kind: EndpointKind::OpenAiCompatible,
            endpoint: "http://127.0.0.1:9".to_string(), // nothing listens here
            api_key: String::new(),
            model: "m".to_string(),
        };
        let err = fetch_models_resolved(&provider).unwrap_err().to_string();
        assert!(err.contains("failed to reach"));
    }
}
