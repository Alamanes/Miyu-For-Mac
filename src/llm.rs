use anyhow::Result;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallDef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(content: &str) -> Self {
        Self {
            role: "system".into(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }
    }
    pub fn user(content: &str) -> Self {
        Self {
            role: "user".into(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }
    }
    pub fn assistant(content: &str, tool_calls: Option<Vec<ToolCallDef>>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.to_string(),
            tool_calls,
            tool_call_id: None,
        }
    }
    pub fn tool(tool_call_id: &str, content: &str) -> Self {
        Self {
            role: "tool".into(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDef {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub def_type: String,
    pub function: ToolFunctionDef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

impl ToolDefinition {
    pub fn new(name: &str, description: &str, parameters: serde_json::Value) -> Self {
        Self {
            def_type: "function".into(),
            function: ToolFunctionDef {
                name: name.to_string(),
                description: description.to_string(),
                parameters,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatResult {
    pub content: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCallDef>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone)]
pub enum StreamChunk {
    Content(String),
    Reasoning(String),
}

#[derive(Clone)]
pub struct OpencodeClient {
    base_url: String,
    model: String,
    client: Client,
}

impl OpencodeClient {
    pub fn new(base_url: Option<&str>, model: Option<&str>) -> Self {
        Self {
            base_url: base_url
                .unwrap_or("https://opencode.ai/zen/v1")
                .trim_end_matches('/')
                .to_string(),
            model: model.unwrap_or("big-pickle").to_string(),
            client: Client::new(),
        }
    }

    pub async fn chat_stream(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        on_chunk: impl Fn(StreamChunk),
    ) -> Result<ChatResult> {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
            "temperature": 0.7,
        });

        if !tools.is_empty() {
            body["tools"] = serde_json::to_value(tools)?;
        }

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .json(&body)
            .send()
            .await?;

        let mut content = String::new();
        let mut reasoning = String::new();
        let mut tool_calls: Vec<ToolCallDef> = Vec::new();
        let mut usage: Option<Usage> = None;

        let mut stream = resp.bytes_stream();
        let mut buf = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buf.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buf.find('\n') {
                let line = buf[..pos].trim().to_string();
                buf = buf[pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }
                let Some(data) = line.strip_prefix("data: ") else {
                    continue;
                };
                if data == "[DONE]" {
                    break;
                }

                let Ok(value) = serde_json::from_str::<serde_json::Value>(data) else {
                    continue;
                };

                if let Some(u) = value.get("usage") {
                    usage = serde_json::from_value(u.clone()).ok();
                }

                let Some(choices) = value.get("choices").and_then(|c| c.as_array()) else {
                    continue;
                };
                let Some(delta) = choices.first().and_then(|c| c.get("delta")) else {
                    continue;
                };

                if let Some(text) = delta.get("content").and_then(|v| v.as_str()) {
                    if !text.is_empty() {
                        content.push_str(text);
                        on_chunk(StreamChunk::Content(text.to_string()));
                    }
                }

                let reasoning_text = delta
                    .get("reasoning_content")
                    .or_else(|| delta.get("reasoning"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !reasoning_text.is_empty() {
                    reasoning.push_str(reasoning_text);
                    on_chunk(StreamChunk::Reasoning(reasoning_text.to_string()));
                }

                if let Some(tcs) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                    for tc in tcs {
                        let idx = tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                        while tool_calls.len() <= idx {
                            tool_calls.push(ToolCallDef {
                                id: String::new(),
                                call_type: "function".into(),
                                function: ToolCallFunction {
                                    name: String::new(),
                                    arguments: String::new(),
                                },
                            });
                        }
                        let entry = &mut tool_calls[idx];
                        if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                            if !id.is_empty() {
                                entry.id = id.to_string();
                            }
                        }
                        if let Some(func) = tc.get("function") {
                            if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                                if !name.is_empty() {
                                    entry.function.name = name.to_string();
                                }
                            }
                            if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                                entry.function.arguments.push_str(args);
                            }
                        }
                    }
                }
            }
        }

        Ok(ChatResult {
            content,
            reasoning: if reasoning.is_empty() {
                None
            } else {
                Some(reasoning)
            },
            tool_calls,
            usage,
        })
    }

    pub async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<ChatResult> {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "temperature": 0.7,
        });
        if !tools.is_empty() {
            body["tools"] = serde_json::to_value(tools)?;
        }

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .json(&body)
            .send()
            .await?;

        let value: serde_json::Value = resp.json().await?;
        let choice = value["choices"]
            .as_array()
            .and_then(|c| c.first())
            .cloned()
            .unwrap_or_default();
        let msg = &choice["message"];

        Ok(ChatResult {
            content: msg["content"].as_str().unwrap_or("").to_string(),
            reasoning: msg
                .get("reasoning_content")
                .or_else(|| msg.get("reasoning"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            tool_calls: msg["tool_calls"]
                .as_array()
                .map(|a| {
                    serde_json::from_value(serde_json::Value::Array(a.clone()))
                        .unwrap_or_default()
                })
                .unwrap_or_default(),
            usage: serde_json::from_value(value["usage"].clone()).ok(),
        })
    }
}
