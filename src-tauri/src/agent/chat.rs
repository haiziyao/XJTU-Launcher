//! Streaming chat against the configured model providers, with a tool loop.
use crate::agent::{
    store::{Conversation, Message, ToolCall},
    tools::ToolDef,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tauri::AppHandle;

const SYSTEM_PROMPT: &str = "你是西安交通大学校园启动器里的 Agent。可以调用工具查询本机已登录账号的校园数据（课表、成绩、考试、考勤、校园卡、体测、空闲教室、全校课程），以及用户启用的 MCP 工具。\
工具结果是唯一的事实来源：查不到或工具报错时如实说明，不要编造课程、成绩或数字。\
用户没有要求写操作时只做查询；评教提交等写操作必须由用户自己在界面中触发。\
用用户的语言简洁回答，涉及列表时用短句或简单列表。";

const MAX_ROUNDS: usize = 8;

#[derive(Clone, Debug)]
pub struct Provider {
    pub protocol: String,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Event {
    Delta {
        request_id: String,
        text: String,
    },
    ToolStart {
        request_id: String,
        call_id: String,
        name: String,
        arguments: String,
    },
    ToolEnd {
        request_id: String,
        call_id: String,
        ok: bool,
        content: String,
    },
    Done {
        request_id: String,
        conversation_id: String,
    },
    Error {
        request_id: String,
        message: String,
    },
    Cancelled {
        request_id: String,
    },
}

pub struct Chat<'a> {
    pub app: AppHandle,
    pub provider: Provider,
    pub tools: Vec<ToolDef>,
    pub request_id: String,
    pub cancel: Arc<AtomicBool>,
    pub emit: &'a mut (dyn FnMut(Event) + Send),
    pub persist: &'a mut (dyn FnMut(&Conversation) + Send),
}

impl Chat<'_> {
    pub async fn run(&mut self, conversation: &mut Conversation) -> Result<(), String> {
        for _ in 0..MAX_ROUNDS {
            if self.cancelled() {
                return Ok(());
            }
            let (text, calls) = self.complete(&conversation.messages).await?;
            let message = Message::assistant(text, calls.clone());
            conversation.messages.push(message);
            (self.persist)(conversation);
            if calls.is_empty() {
                return Ok(());
            }
            for call in calls {
                if self.cancelled() {
                    return Ok(());
                }
                (self.emit)(Event::ToolStart {
                    request_id: self.request_id.clone(),
                    call_id: call.id.clone(),
                    name: call.name.clone(),
                    arguments: call.arguments.clone(),
                });
                let args: Value = serde_json::from_str(&call.arguments).unwrap_or(json!({}));
                let outcome = match self.tools.iter().find(|tool| tool.name == call.name) {
                    Some(tool) => tool.call(&self.app, args).await,
                    None => Err(format!("工具 {} 不存在", call.name)),
                };
                let (content, failed) = match outcome {
                    Ok(content) => (content, false),
                    Err(error) => (error, true),
                };
                (self.emit)(Event::ToolEnd {
                    request_id: self.request_id.clone(),
                    call_id: call.id.clone(),
                    ok: !failed,
                    content: content.clone(),
                });
                conversation
                    .messages
                    .push(Message::tool(&call, content, failed));
                (self.persist)(conversation);
            }
        }
        Err("工具调用轮次过多，请换个问法".into())
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    async fn complete(&mut self, messages: &[Message]) -> Result<(String, Vec<ToolCall>), String> {
        if !self.provider.api_key.trim().is_empty() && self.provider.base_url.is_empty() {
            return Err("模型服务地址为空".into());
        }
        match self.provider.protocol.as_str() {
            "openai-compatible" => self.openai(messages).await,
            "anthropic" => self.anthropic(messages).await,
            "gemini" => self.gemini(messages).await,
            _ => Err("不支持的模型协议".into()),
        }
    }

    async fn openai(&mut self, messages: &[Message]) -> Result<(String, Vec<ToolCall>), String> {
        let mut wire = vec![json!({ "role": "system", "content": SYSTEM_PROMPT })];
        for message in messages {
            wire.push(match message.role.as_str() {
                "assistant" => {
                    let mut value = json!({ "role": "assistant", "content": message.content });
                    if let Some(calls) = &message.tool_calls {
                        value["tool_calls"] = Value::Array(
                            calls
                                .iter()
                                .map(|call| {
                                    json!({
                                        "id": call.id,
                                        "type": "function",
                                        "function": { "name": call.name, "arguments": call.arguments }
                                    })
                                })
                                .collect(),
                        );
                    }
                    value
                }
                "tool" => json!({
                    "role": "tool",
                    "tool_call_id": message.tool_call_id,
                    "content": message.content
                }),
                _ => json!({ "role": "user", "content": message.content }),
            });
        }
        let mut body = json!({
            "model": self.provider.model,
            "stream": true,
            "messages": wire,
        });
        if !self.tools.is_empty() {
            body["tools"] = Value::Array(
                self.tools
                    .iter()
                    .map(|tool| {
                        json!({
                            "type": "function",
                            "function": {
                                "name": tool.name,
                                "description": tool.description,
                                "parameters": tool.parameters
                            }
                        })
                    })
                    .collect(),
            );
        }
        let request = http_client()?
            .post(format!("{}/chat/completions", base(&self.provider)?))
            .bearer_auth(&self.provider.api_key)
            .json(&body);
        let response = request.send().await.map_err(connect_error)?;
        let response = check(response).await?;
        let mut text = String::new();
        let mut calls: BTreeMap<usize, (String, String, String)> = BTreeMap::new();
        let request_id = self.request_id.clone();
        let cancel = self.cancel.clone();
        let emit = &mut *self.emit;
        stream_sse(response, &cancel, |_, data| {
            if data.trim() == "[DONE]" {
                return Ok(());
            }
            let Ok(value) = serde_json::from_str::<Value>(data) else {
                return Ok(());
            };
            let Some(delta) = value
                .get("choices")
                .and_then(Value::as_array)
                .and_then(|choices| choices.first())
                .and_then(|choice| choice.get("delta"))
            else {
                return Ok(());
            };
            if let Some(chunk) = delta.get("content").and_then(Value::as_str) {
                if !chunk.is_empty() {
                    text.push_str(chunk);
                    (emit)(Event::Delta {
                        request_id: request_id.clone(),
                        text: chunk.to_string(),
                    });
                }
            }
            if let Some(items) = delta.get("tool_calls").and_then(Value::as_array) {
                for item in items {
                    let index = item.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
                    let entry = calls
                        .entry(index)
                        .or_insert_with(|| (String::new(), String::new(), String::new()));
                    if let Some(id) = item.get("id").and_then(Value::as_str) {
                        entry.0 = id.to_string();
                    }
                    if let Some(function) = item.get("function") {
                        if let Some(name) = function.get("name").and_then(Value::as_str) {
                            entry.1.push_str(name);
                        }
                        if let Some(arguments) = function.get("arguments").and_then(Value::as_str) {
                            entry.2.push_str(arguments);
                        }
                    }
                }
            }
            Ok(())
        })
        .await?;
        let calls = calls
            .into_iter()
            .map(|(index, (id, name, arguments))| ToolCall {
                id: if id.is_empty() {
                    format!("call_{index}")
                } else {
                    id
                },
                name,
                arguments: if arguments.is_empty() {
                    "{}".into()
                } else {
                    arguments
                },
            })
            .collect();
        Ok((text, calls))
    }

    async fn anthropic(&mut self, messages: &[Message]) -> Result<(String, Vec<ToolCall>), String> {
        let mut wire: Vec<Value> = Vec::new();
        let mut pending_tools: Vec<Value> = Vec::new();
        for message in messages {
            if message.role == "tool" {
                pending_tools.push(json!({
                    "type": "tool_result",
                    "tool_use_id": message.tool_call_id,
                    "content": message.content,
                    "is_error": message.error
                }));
                continue;
            }
            if !pending_tools.is_empty() {
                wire.push(json!({ "role": "user", "content": pending_tools }));
                pending_tools.clear();
            }
            match message.role.as_str() {
                "assistant" => {
                    let mut blocks = Vec::new();
                    if !message.content.is_empty() {
                        blocks.push(json!({ "type": "text", "text": message.content }));
                    }
                    for call in message.tool_calls.iter().flatten() {
                        let input: Value =
                            serde_json::from_str(&call.arguments).unwrap_or(json!({}));
                        blocks.push(json!({
                            "type": "tool_use",
                            "id": call.id,
                            "name": call.name,
                            "input": input
                        }));
                    }
                    if !blocks.is_empty() {
                        wire.push(json!({ "role": "assistant", "content": blocks }));
                    }
                }
                _ => wire.push(json!({
                    "role": "user",
                    "content": [{ "type": "text", "text": message.content }]
                })),
            }
        }
        if !pending_tools.is_empty() {
            wire.push(json!({ "role": "user", "content": pending_tools }));
        }
        let mut body = json!({
            "model": self.provider.model,
            "max_tokens": 4096,
            "stream": true,
            "system": SYSTEM_PROMPT,
            "messages": wire,
        });
        if !self.tools.is_empty() {
            body["tools"] = Value::Array(
                self.tools
                    .iter()
                    .map(|tool| {
                        json!({
                            "name": tool.name,
                            "description": tool.description,
                            "input_schema": tool.parameters
                        })
                    })
                    .collect(),
            );
        }
        let request = http_client()?
            .post(format!("{}/messages", base(&self.provider)?))
            .header("x-api-key", &self.provider.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body);
        let response = request.send().await.map_err(connect_error)?;
        let response = check(response).await?;
        let mut text = String::new();
        let mut calls: BTreeMap<usize, (String, String, String)> = BTreeMap::new();
        let request_id = self.request_id.clone();
        let cancel = self.cancel.clone();
        let emit = &mut *self.emit;
        stream_sse(response, &cancel, |kind, data| {
            let Ok(value) = serde_json::from_str::<Value>(data) else {
                return Ok(());
            };
            let event = value.get("type").and_then(Value::as_str).unwrap_or(kind);
            match event {
                "content_block_start" => {
                    if value["content_block"]["type"] == "tool_use" {
                        let index = value["index"].as_u64().unwrap_or(0) as usize;
                        calls.insert(
                            index,
                            (
                                value["content_block"]["id"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                                value["content_block"]["name"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                                String::new(),
                            ),
                        );
                    }
                }
                "content_block_delta" => {
                    if let Some(chunk) = value["delta"]["text"].as_str() {
                        if !chunk.is_empty() {
                            text.push_str(chunk);
                            (emit)(Event::Delta {
                                request_id: request_id.clone(),
                                text: chunk.to_string(),
                            });
                        }
                    }
                    if let Some(partial) = value["delta"]["partial_json"].as_str() {
                        let index = value["index"].as_u64().unwrap_or(0) as usize;
                        if let Some(entry) = calls.get_mut(&index) {
                            entry.2.push_str(partial);
                        }
                    }
                }
                "error" => {
                    return Err(value["error"]["message"]
                        .as_str()
                        .unwrap_or("模型服务返回错误")
                        .to_string())
                }
                _ => {}
            }
            Ok(())
        })
        .await?;
        let calls = calls
            .into_iter()
            .filter(|(_, (_, name, _))| !name.is_empty())
            .map(|(index, (id, name, arguments))| ToolCall {
                id: if id.is_empty() {
                    format!("call_{index}")
                } else {
                    id
                },
                name,
                arguments: if arguments.trim().is_empty() {
                    "{}".into()
                } else {
                    arguments
                },
            })
            .collect();
        Ok((text, calls))
    }

    async fn gemini(&mut self, messages: &[Message]) -> Result<(String, Vec<ToolCall>), String> {
        let mut contents: Vec<Value> = Vec::new();
        let mut pending_tools: Vec<Value> = Vec::new();
        for message in messages {
            if message.role == "tool" {
                pending_tools.push(json!({
                    "functionResponse": {
                        "name": message.name.clone().unwrap_or_default(),
                        "response": { "result": message.content }
                    }
                }));
                continue;
            }
            if !pending_tools.is_empty() {
                contents.push(json!({ "role": "user", "parts": pending_tools }));
                pending_tools.clear();
            }
            match message.role.as_str() {
                "assistant" => {
                    let mut parts = Vec::new();
                    if !message.content.is_empty() {
                        parts.push(json!({ "text": message.content }));
                    }
                    for call in message.tool_calls.iter().flatten() {
                        let args: Value =
                            serde_json::from_str(&call.arguments).unwrap_or(json!({}));
                        parts.push(json!({
                            "functionCall": { "name": call.name, "args": args }
                        }));
                    }
                    if !parts.is_empty() {
                        contents.push(json!({ "role": "model", "parts": parts }));
                    }
                }
                _ => contents.push(json!({
                    "role": "user",
                    "parts": [{ "text": message.content }]
                })),
            }
        }
        if !pending_tools.is_empty() {
            contents.push(json!({ "role": "user", "parts": pending_tools }));
        }
        let mut body = json!({
            "systemInstruction": { "parts": [{ "text": SYSTEM_PROMPT }] },
            "contents": contents,
        });
        if !self.tools.is_empty() {
            body["tools"] = json!([{
                "functionDeclarations": self.tools.iter().map(|tool| json!({
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": gemini_schema(&tool.parameters)
                })).collect::<Vec<_>>()
            }]);
        }
        let model = self.provider.model.trim_start_matches("models/");
        let url = url::Url::parse(&format!(
            "{}/models/{}:streamGenerateContent",
            base(&self.provider)?,
            model
        ))
        .map_err(|_| "模型服务地址无效")?;
        let mut url = url;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("alt", "sse");
            if !self.provider.api_key.is_empty() {
                query.append_pair("key", &self.provider.api_key);
            }
        }
        let request = http_client()?.post(url).json(&body);
        let response = request.send().await.map_err(connect_error)?;
        let response = check(response).await?;
        let mut text = String::new();
        let mut calls: Vec<ToolCall> = Vec::new();
        let request_id = self.request_id.clone();
        let cancel = self.cancel.clone();
        let emit = &mut *self.emit;
        stream_sse(response, &cancel, |_, data| {
            let Ok(value) = serde_json::from_str::<Value>(data) else {
                return Ok(());
            };
            let Some(parts) = value
                .get("candidates")
                .and_then(Value::as_array)
                .and_then(|candidates| candidates.first())
                .and_then(|candidate| candidate.get("content"))
                .and_then(|content| content.get("parts"))
                .and_then(Value::as_array)
            else {
                return Ok(());
            };
            for part in parts {
                if let Some(chunk) = part.get("text").and_then(Value::as_str) {
                    if !chunk.is_empty() {
                        text.push_str(chunk);
                        (emit)(Event::Delta {
                            request_id: request_id.clone(),
                            text: chunk.to_string(),
                        });
                    }
                }
                if let Some(call) = part.get("functionCall") {
                    let name = call
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    if name.is_empty() {
                        continue;
                    }
                    let arguments = call.get("args").cloned().unwrap_or(json!({}));
                    calls.push(ToolCall {
                        id: format!("call_{}", calls.len() + 1),
                        name,
                        arguments: arguments.to_string(),
                    });
                }
            }
            Ok(())
        })
        .await?;
        Ok((text, calls))
    }
}

fn base(provider: &Provider) -> Result<String, String> {
    let value = provider.base_url.trim().trim_end_matches('/');
    if value.is_empty() {
        return Err("模型服务地址为空".into());
    }
    Ok(value.to_string())
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("XJTU-Launcher/agent")
        .build()
        .map_err(|_| "无法初始化模型连接".to_string())
}

fn connect_error(_: reqwest::Error) -> String {
    "无法连接模型服务，请检查接口地址和网络".into()
}

async fn check(response: reqwest::Response) -> Result<reqwest::Response, String> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    let detail: String = body.chars().take(300).collect();
    Err(match status.as_u16() {
        401 | 403 => "API Key 无效，或没有调用该模型的权限".to_string(),
        404 => "模型或接口地址不存在，请检查模型配置".to_string(),
        429 => "模型服务限流，请稍后重试".to_string(),
        code => format!("模型服务返回 HTTP {code}：{detail}"),
    })
}

/// Streams Server-Sent Events, calling `on_event(name, data)` for each event.
async fn stream_sse(
    mut response: reqwest::Response,
    cancel: &Arc<AtomicBool>,
    mut on_event: impl FnMut(&str, &str) -> Result<(), String>,
) -> Result<(), String> {
    let mut decoder = SseDecoder::default();
    while let Some(chunk) = response.chunk().await.map_err(|_| "模型响应中断")? {
        if cancel.load(Ordering::Relaxed) {
            return Err("__cancelled__".into());
        }
        for (event, data) in decoder.push(&chunk) {
            on_event(&event, &data)?;
        }
    }
    for (event, data) in decoder.finish() {
        on_event(&event, &data)?;
    }
    Ok(())
}

#[derive(Default)]
struct SseDecoder {
    buffer: Vec<u8>,
    event: String,
    data: String,
}

impl SseDecoder {
    fn push(&mut self, chunk: &[u8]) -> Vec<(String, String)> {
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();
        while let Some(position) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.buffer.drain(..=position).collect();
            let line = String::from_utf8_lossy(&line[..line.len().saturating_sub(1)])
                .trim_end_matches('\r')
                .to_string();
            self.line(&line, &mut events);
        }
        events
    }

    fn line(&mut self, line: &str, events: &mut Vec<(String, String)>) {
        if line.is_empty() {
            if !self.data.is_empty() {
                events.push((
                    std::mem::take(&mut self.event),
                    std::mem::take(&mut self.data),
                ));
            } else {
                self.event.clear();
            }
        } else if let Some(rest) = line.strip_prefix("event:") {
            self.event = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("data:") {
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            if !self.data.is_empty() {
                self.data.push('\n');
            }
            self.data.push_str(rest);
        }
    }

    fn finish(&mut self) -> Vec<(String, String)> {
        if self.buffer.is_empty() {
            return Vec::new();
        }
        let rest = String::from_utf8_lossy(&self.buffer).to_string();
        self.buffer.clear();
        let mut events = Vec::new();
        self.line(&rest, &mut events);
        self.line("", &mut events);
        events
    }
}

/// Gemini rejects JSON Schema keywords it does not model.
fn gemini_schema(schema: &Value) -> Value {
    let mut schema = schema.clone();
    if let Some(object) = schema.as_object_mut() {
        object.remove("additionalProperties");
        object.remove("$schema");
    }
    schema
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_sse_frames_across_chunks() {
        let mut decoder = SseDecoder::default();
        let mut events = decoder.push(b"event: message\ndata: {\"a\":");
        assert!(events.is_empty());
        events.extend(decoder.push(b"1}\n\n"));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "message");
        assert_eq!(events[0].1, "{\"a\":1}");
    }

    #[test]
    fn decodes_multiline_data() {
        let mut decoder = SseDecoder::default();
        let events = decoder.push(b"data: one\ndata: two\n\n");
        assert_eq!(events[0].1, "one\ntwo");
    }

    #[test]
    fn strips_gemini_unsupported_keywords() {
        let schema = json!({"type": "object", "properties": {}, "additionalProperties": false});
        let cleaned = gemini_schema(&schema);
        assert!(cleaned.get("additionalProperties").is_none());
    }
}
