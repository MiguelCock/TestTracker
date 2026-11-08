use std::time::Duration;

use reqwest::StatusCode;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{Value, json};
use url::Url;

use crate::ai::model::{AiProviderKind, ProviderReply};
use crate::error::{AppError, AppResult};

const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

pub fn validate_base_url(base_url: &str) -> AppResult<()> {
    let url = Url::parse(base_url.trim())
        .map_err(|_| AppError::Validation("Enter a valid provider URL.".into()))?;
    if url.host_str().is_none() {
        return Err(AppError::Validation(
            "The provider URL must include a host.".into(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::Validation(
            "Provider URLs must not contain credentials.".into(),
        ));
    }
    match url.scheme() {
        "https" => Ok(()),
        "http" if is_local_host(url.host_str()) => Ok(()),
        _ => Err(AppError::Validation(
            "Provider URLs must use HTTPS, except localhost.".into(),
        )),
    }
}

pub fn is_local(_kind: AiProviderKind, base_url: &str) -> bool {
    Url::parse(base_url)
        .ok()
        .and_then(|url| is_local_host(url.host_str()).then_some(()))
        .is_some()
}

fn is_local_host(host: Option<&str>) -> bool {
    let Some(host) = host else {
        return false;
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

pub struct ProviderRequest<'a> {
    pub kind: AiProviderKind,
    pub base_url: &'a str,
    pub model: &'a str,
    pub api_key: &'a str,
    pub system_prompt: &'a str,
    pub prompt: &'a str,
}

pub async fn send(request: &ProviderRequest<'_>) -> AppResult<ProviderReply> {
    let endpoint = endpoint(request.kind, request.base_url);
    let body = request_body(
        request.kind,
        request.model,
        request.system_prompt,
        request.prompt,
    );
    let local = is_local(request.kind, request.base_url);
    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("TestTracker/0.1");
    if local {
        builder = builder.no_proxy();
    }
    let client = builder
        .build()
        .map_err(|_| AppError::External("Unable to initialize the provider client.".into()))?;

    let mut http_request = client.post(&endpoint).json(&body);
    let key = request.api_key;
    match request.kind {
        AiProviderKind::Anthropic => {
            http_request = http_request
                .header("x-api-key", key)
                .header("anthropic-version", "2023-06-01");
        }
        AiProviderKind::AzureOpenai => {
            if !key.is_empty() {
                http_request = http_request.header("api-key", key);
            }
        }
        AiProviderKind::OpenaiCompatible | AiProviderKind::Ollama => {
            if !key.is_empty() {
                http_request = http_request.header(AUTHORIZATION, format!("Bearer {key}"));
            }
        }
    }

    let response = http_request
        .header(CONTENT_TYPE, "application/json")
        .send()
        .await
        .map_err(|_| AppError::External("Unable to contact the AI provider.".into()))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.bytes().await.unwrap_or_default();
        return Err(provider_status_error(status, &body));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|_| AppError::External("Unable to read the provider response.".into()))?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(AppError::External(
            "The provider response is too large.".into(),
        ));
    }
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::External("The provider returned invalid JSON.".into()))?;
    parse_response(request.kind, &value)
}

fn provider_status_error(status: StatusCode, body: &[u8]) -> AppError {
    if body.len() <= 64 * 1024
        && let Ok(value) = serde_json::from_slice::<Value>(body)
    {
        let detail = value
            .get("error")
            .or_else(|| value.get("message"))
            .and_then(Value::as_str)
            .map(|message| message.trim())
            .filter(|message| !message.is_empty());
        if let Some(detail) = detail {
            let detail: String = detail.chars().take(300).collect();
            return AppError::External(format!("{detail} (HTTP {})", status.as_u16()));
        }
    }
    let message = match status.as_u16() {
        401 | 403 => "Provider authentication failed. Check the provider API key.",
        404 => "Provider endpoint not found. Check the provider URL and model.",
        429 => "Provider rate limit reached. Try again later.",
        500..=599 => "The provider is currently unavailable.",
        _ => "The provider request failed.",
    };
    AppError::External(format!("{message} (HTTP {})", status.as_u16()))
}

pub async fn list_models(
    kind: AiProviderKind,
    base_url: &str,
    api_key: &str,
) -> AppResult<Vec<String>> {
    let endpoint = models_endpoint(kind, base_url)?;
    let local = is_local(kind, base_url);
    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("TestTracker/0.1");
    if local {
        builder = builder.no_proxy();
    }
    let client = builder
        .build()
        .map_err(|_| AppError::External("Unable to initialize the provider client.".into()))?;
    let mut request = client.get(endpoint);
    match kind {
        AiProviderKind::Anthropic => {
            request = request
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01");
        }
        AiProviderKind::AzureOpenai => {
            return Err(AppError::Validation(
                "Model discovery is not available for Azure OpenAI. Enter the deployment name manually."
                    .into(),
            ));
        }
        AiProviderKind::OpenaiCompatible => {
            if !api_key.is_empty() {
                request = request.header(AUTHORIZATION, format!("Bearer {api_key}"));
            }
        }
        AiProviderKind::Ollama => {
            if !api_key.is_empty() {
                request = request.header(AUTHORIZATION, format!("Bearer {api_key}"));
            }
        }
    }
    let response = request
        .send()
        .await
        .map_err(|_| AppError::External("Unable to contact the AI provider.".into()))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.bytes().await.unwrap_or_default();
        return Err(provider_status_error(status, &body));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|_| AppError::External("Unable to read the provider response.".into()))?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(AppError::External(
            "The provider response is too large.".into(),
        ));
    }
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::External("The provider returned invalid JSON.".into()))?;
    let mut models = parse_models(kind, &value)?;
    models.sort();
    models.dedup();
    models.truncate(200);
    Ok(models)
}

fn models_endpoint(kind: AiProviderKind, base_url: &str) -> AppResult<String> {
    let base = base_url.trim().trim_end_matches('/');
    match kind {
        AiProviderKind::Ollama => {
            if base.ends_with("/api/tags") {
                Ok(base.to_string())
            } else {
                Ok(format!("{base}/api/tags"))
            }
        }
        AiProviderKind::OpenaiCompatible => {
            if base.ends_with("/models") {
                Ok(base.to_string())
            } else if base.ends_with("/v1") {
                Ok(format!("{base}/models"))
            } else {
                Ok(format!("{base}/v1/models"))
            }
        }
        AiProviderKind::Anthropic => {
            if base.ends_with("/v1/models") {
                Ok(base.to_string())
            } else {
                Ok(format!("{base}/v1/models"))
            }
        }
        AiProviderKind::AzureOpenai => Err(AppError::Validation(
            "Model discovery is not available for Azure OpenAI.".into(),
        )),
    }
}

fn parse_models(kind: AiProviderKind, value: &Value) -> AppResult<Vec<String>> {
    let models: Vec<String> = match kind {
        AiProviderKind::Ollama => value
            .get("models")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        item.get("name")
                            .or_else(|| item.get("model"))
                            .and_then(Value::as_str)
                    })
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        AiProviderKind::OpenaiCompatible | AiProviderKind::Anthropic => value
            .get("data")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("id").and_then(Value::as_str))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        AiProviderKind::AzureOpenai => Vec::new(),
    };
    if models.is_empty() {
        return Ok(Vec::new());
    }
    Ok(models)
}

pub fn endpoint(kind: AiProviderKind, base_url: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    match kind {
        AiProviderKind::Ollama => {
            if base.ends_with("/api/chat") {
                base.to_string()
            } else {
                format!("{base}/api/chat")
            }
        }
        AiProviderKind::OpenaiCompatible => {
            if base.ends_with("/chat/completions") {
                base.to_string()
            } else if base.ends_with("/v1") {
                format!("{base}/chat/completions")
            } else {
                format!("{base}/v1/chat/completions")
            }
        }
        AiProviderKind::Anthropic => {
            if base.ends_with("/v1/messages") {
                base.to_string()
            } else {
                format!("{base}/v1/messages")
            }
        }
        AiProviderKind::AzureOpenai => base.to_string(),
    }
}

pub fn request_body(kind: AiProviderKind, model: &str, system: &str, prompt: &str) -> Value {
    match kind {
        AiProviderKind::Ollama => json!({
            "model": model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": prompt }
            ],
            "stream": false
        }),
        AiProviderKind::OpenaiCompatible | AiProviderKind::AzureOpenai => json!({
            "model": model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": prompt }
            ],
            "temperature": 0.2
        }),
        AiProviderKind::Anthropic => json!({
            "model": model,
            "max_tokens": 4096,
            "temperature": 0.2,
            "system": system,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        }),
    }
}

pub(crate) fn parse_response(kind: AiProviderKind, value: &Value) -> AppResult<ProviderReply> {
    let (content, prompt_tokens, completion_tokens) = match kind {
        AiProviderKind::OpenaiCompatible | AiProviderKind::AzureOpenai => (
            value
                .pointer("/choices/0/message/content")
                .and_then(Value::as_str),
            value
                .pointer("/usage/prompt_tokens")
                .and_then(Value::as_u64),
            value
                .pointer("/usage/completion_tokens")
                .and_then(Value::as_u64),
        ),
        AiProviderKind::Ollama => (
            value.pointer("/message/content").and_then(Value::as_str),
            value.pointer("/prompt_eval_count").and_then(Value::as_u64),
            value.pointer("/eval_count").and_then(Value::as_u64),
        ),
        AiProviderKind::Anthropic => (
            value.pointer("/content/0/text").and_then(Value::as_str),
            value.pointer("/usage/input_tokens").and_then(Value::as_u64),
            value
                .pointer("/usage/output_tokens")
                .and_then(Value::as_u64),
        ),
    };
    let content = content
        .map(str::trim)
        .filter(|content| !content.is_empty())
        .ok_or_else(|| AppError::External("The provider returned an empty response.".into()))?;
    Ok(ProviderReply {
        content: content.to_string(),
        prompt_tokens,
        completion_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_endpoints() {
        assert_eq!(
            endpoint(AiProviderKind::Ollama, "http://localhost:11434"),
            "http://localhost:11434/api/chat"
        );
        assert_eq!(
            endpoint(AiProviderKind::Ollama, "http://localhost:11434/api/chat"),
            "http://localhost:11434/api/chat"
        );
        assert_eq!(
            endpoint(
                AiProviderKind::OpenaiCompatible,
                "https://api.example.com/v1"
            ),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            endpoint(AiProviderKind::Anthropic, "https://api.anthropic.com"),
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(
            endpoint(
                AiProviderKind::AzureOpenai,
                "https://example.openai.azure.com/openai/deployments/dep/chat/completions?api-version=2024-10-21"
            ),
            "https://example.openai.azure.com/openai/deployments/dep/chat/completions?api-version=2024-10-21"
        );
    }

    #[test]
    fn validates_urls() {
        assert!(validate_base_url("https://api.openai.com/v1").is_ok());
        assert!(validate_base_url("http://localhost:11434").is_ok());
        assert!(validate_base_url("http://127.0.0.1:11434").is_ok());
        assert!(validate_base_url("http://api.openai.com/v1").is_err());
        assert!(validate_base_url("https://user:pass@api.example.com/v1").is_err());
        assert!(validate_base_url("ftp://example.com").is_err());
        assert!(validate_base_url("not a url").is_err());
    }

    #[test]
    fn detects_local_providers() {
        assert!(is_local(AiProviderKind::Ollama, "http://localhost:11434"));
        assert!(!is_local(AiProviderKind::Ollama, "https://example.com"));
        assert!(is_local(
            AiProviderKind::OpenaiCompatible,
            "http://localhost:1234/v1"
        ));
        assert!(!is_local(
            AiProviderKind::OpenaiCompatible,
            "https://api.openai.com/v1"
        ));
    }

    #[test]
    fn parses_provider_responses() {
        let openai = json!({"choices":[{"message":{"content":"OpenAI answer"}}],"usage":{"prompt_tokens":10,"completion_tokens":5}});
        let reply = parse_response(AiProviderKind::OpenaiCompatible, &openai).expect("openai");
        assert_eq!(reply.content, "OpenAI answer");
        assert_eq!(reply.prompt_tokens, Some(10));

        let ollama =
            json!({"message":{"content":"Ollama answer"},"prompt_eval_count":3,"eval_count":7});
        let reply = parse_response(AiProviderKind::Ollama, &ollama).expect("ollama");
        assert_eq!(reply.content, "Ollama answer");
        assert_eq!(reply.completion_tokens, Some(7));

        let anthropic = json!({"content":[{"text":"Anthropic answer"}],"usage":{"input_tokens":4,"output_tokens":9}});
        let reply = parse_response(AiProviderKind::Anthropic, &anthropic).expect("anthropic");
        assert_eq!(reply.content, "Anthropic answer");
        assert_eq!(reply.prompt_tokens, Some(4));
    }

    #[test]
    fn parses_provider_errors() {
        let error = provider_status_error(
            StatusCode::NOT_FOUND,
            br#"{"error":"model \"llama3.2\" not found, try pulling it first"}"#,
        );
        let message = error.to_string();
        assert!(message.contains("llama3.2"));
        assert!(message.contains("404"));
    }

    #[test]
    fn builds_model_endpoints() {
        assert_eq!(
            models_endpoint(AiProviderKind::Ollama, "http://localhost:11434").unwrap(),
            "http://localhost:11434/api/tags"
        );
        assert_eq!(
            models_endpoint(
                AiProviderKind::OpenaiCompatible,
                "https://api.example.com/v1"
            )
            .unwrap(),
            "https://api.example.com/v1/models"
        );
        assert_eq!(
            models_endpoint(AiProviderKind::Anthropic, "https://api.anthropic.com").unwrap(),
            "https://api.anthropic.com/v1/models"
        );
    }

    #[test]
    fn parses_model_lists() {
        let ollama = json!({"models":[{"name":"llama3.2:latest"},{"name":"qwen2.5:1.5b"}]});
        let models = parse_models(AiProviderKind::Ollama, &ollama).expect("ollama");
        assert_eq!(models.len(), 2);
        let openai = json!({"data":[{"id":"gpt-4.1"},{"id":"gpt-4.1-mini"}]});
        let models = parse_models(AiProviderKind::OpenaiCompatible, &openai).expect("openai");
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn rejects_empty_provider_response() {
        let value = json!({"choices":[{"message":{"content":""}}]});
        assert!(parse_response(AiProviderKind::OpenaiCompatible, &value).is_err());
    }
}
