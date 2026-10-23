//! Loopback MCP endpoint for the personal page.
//!
//! This is the opposite direction from `mcp.rs`: instead of connecting to
//! external MCP servers, it offers the launcher's campus tools to coding
//! agents. There is no request authentication by user decision, so the listener
//! is bound to 127.0.0.1 only. Today every tool is a query; write tools can be
//! added later without changing the transport.
use crate::{agent::tools, config, config_path};
use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited};
use hyper::{body::Incoming, header, service::service_fn, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde_json::{json, Value};
use std::{net::TcpListener, sync::Mutex};
use tauri::AppHandle;
use tokio::{net::TcpListener as AsyncListener, sync::oneshot};

const ENDPOINT: &str = "/mcp";
/// A campus tool request is small; refuse anything that could be a body dump.
const BODY_LIMIT: usize = 256 * 1024;

#[derive(Default)]
pub struct State {
    running: Mutex<Option<Running>>,
    error: Mutex<Option<String>>,
}

struct Running {
    port: u16,
    shutdown: Option<oneshot::Sender<()>>,
}

#[derive(Clone)]
struct ServerState {
    app: AppHandle,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub running: bool,
    pub enabled: bool,
    pub address: String,
    pub error: Option<String>,
}

#[derive(serde::Serialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    /// Small display group in the page's tool list.
    pub category: String,
}

pub fn address(port: u16) -> String {
    format!("http://127.0.0.1:{port}{ENDPOINT}")
}

impl State {
    /// The address reflects the listener that is actually bound, so a port
    /// change is not claimed before the new socket is open.
    pub fn status(&self, enabled: bool, configured_port: u16) -> Status {
        let port = self
            .running
            .lock()
            .ok()
            .and_then(|running| running.as_ref().map(|running| running.port));
        Status {
            running: port.is_some(),
            enabled,
            address: address(port.unwrap_or(configured_port)),
            error: self.error.lock().ok().and_then(|error| error.clone()),
        }
    }

    /// Port of the live listener, if any. Used to restore the old endpoint when
    /// a requested change fails.
    pub fn running_port(&self) -> Option<u16> {
        self.running
            .lock()
            .ok()
            .and_then(|running| running.as_ref().map(|running| running.port))
    }

    fn set_error(&self, message: Option<String>) {
        if let Ok(mut error) = self.error.lock() {
            *error = message;
        }
    }
}

/// Loopback bind shared by the startup and port-change paths. Binding happens
/// on the caller's thread so a port that is already taken fails the request
/// instead of silently serving somewhere else.
fn bind(port: u16) -> Result<TcpListener, String> {
    TcpListener::bind(("127.0.0.1", port))
        .map_err(|error| format!("无法监听本机端口 {port}：{error}"))
}

/// Bind and serve on the calling thread's runtime. A bind failure is returned
/// before the listener is handed to the background task, so the caller can show
/// the real reason (usually another process on the port).
pub fn start(app: &AppHandle, state: &State, port: u16) -> Result<(), String> {
    let mut running = state.running.lock().map_err(|_| "MCP 服务状态不可用")?;
    if let Some(current) = running.as_ref() {
        if current.port == port {
            return Ok(());
        }
        let message = format!("MCP 服务已在端口 {} 运行", current.port);
        state.set_error(Some(message.clone()));
        return Err(message);
    }
    let listener = match bind(port) {
        Ok(listener) => listener,
        Err(message) => {
            state.set_error(Some(message.clone()));
            return Err(message);
        }
    };
    if let Err(error) = listener.set_nonblocking(true) {
        let message = format!("无法启动 MCP 服务：{error}");
        state.set_error(Some(message.clone()));
        return Err(message);
    }
    let (shutdown, mut stop) = oneshot::channel();
    let server = ServerState { app: app.clone() };
    tauri::async_runtime::spawn(async move {
        let Ok(listener) = AsyncListener::from_std(listener) else {
            return;
        };
        loop {
            tokio::select! {
                _ = &mut stop => break,
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { continue };
                    let server = server.clone();
                    tauri::async_runtime::spawn(async move {
                        let service = service_fn(move |request: Request<Incoming>| {
                            let server = server.clone();
                            async move {
                                Ok::<_, std::convert::Infallible>(handle(server, request).await)
                            }
                        });
                        let _ = hyper::server::conn::http1::Builder::new()
                            .serve_connection(TokioIo::new(stream), service)
                            .await;
                    });
                }
            }
        }
    });
    *running = Some(Running {
        port,
        shutdown: Some(shutdown),
    });
    state.set_error(None);
    Ok(())
}

pub fn stop(state: &State) {
    if let Ok(mut running) = state.running.lock() {
        if let Some(mut current) = running.take() {
            if let Some(shutdown) = current.shutdown.take() {
                let _ = shutdown.send(());
            }
        }
    }
    state.set_error(None);
}

/// Every built-in campus tool, including the ones the user currently keeps out.
/// The page needs the full list to offer checkboxes.
pub fn tools() -> Vec<Tool> {
    tools::builtin()
        .into_iter()
        .map(|tool| Tool {
            category: category(&tool.name).into(),
            name: tool.name,
            description: tool.description,
        })
        .collect()
}

/// Kept in the backend so the page and `tools/list` describe the same tools.
fn category(name: &str) -> &'static str {
    match name {
        "campus_timetable" | "campus_terms" | "campus_grades" | "campus_exams"
        | "campus_courses" => "课程与成绩",
        "campus_attendance_streams" | "campus_attendance_records" => "考勤",
        "campus_card" | "campus_card_transactions" => "校园卡",
        "campus_fitness" | "campus_fitness_score" => "体测",
        "campus_evaluations" => "评教",
        "campus_profile" => "学籍档案",
        "campus_lms_courses"
        | "campus_lms_activities"
        | "campus_lms_activity"
        | "campus_lms_submissions"
        | "campus_lms_submit" => "思源学堂",
        "campus_library_floors"
        | "campus_library_areas"
        | "campus_library_seats"
        | "campus_venues"
        | "campus_venue_slots"
        | "campus_venue_orders" => "图书馆与场馆",
        _ => "教室与校历",
    }
}

type Reply = Response<Full<Bytes>>;

async fn handle(state: ServerState, request: Request<Incoming>) -> Reply {
    if request.uri().path() != ENDPOINT {
        return text(StatusCode::NOT_FOUND, "未知路径");
    }
    if request.method() != Method::POST {
        return text(StatusCode::METHOD_NOT_ALLOWED, "仅支持 POST /mcp");
    }
    let body = match Limited::new(request.into_body(), BODY_LIMIT)
        .collect()
        .await
    {
        Ok(body) => body.to_bytes(),
        Err(_) => return text(StatusCode::PAYLOAD_TOO_LARGE, "请求内容过大或无法读取"),
    };
    let Ok(request) = serde_json::from_slice::<Value>(&body) else {
        return reply(rpc_error(Value::Null, -32700, "请求不是有效的 JSON".into()));
    };
    if let Value::Array(requests) = request {
        if requests.is_empty() {
            return reply(rpc_error(Value::Null, -32600, "批量请求不能为空".into()));
        }
        let mut answers = Vec::new();
        for request in requests {
            if let Some(answer) = respond(&state, request).await {
                answers.push(answer);
            }
        }
        // Every entry was a notification.
        if answers.is_empty() {
            return accepted();
        }
        return reply(Value::Array(answers));
    }
    match respond(&state, request).await {
        Some(answer) => reply(answer),
        None => accepted(),
    }
}

async fn respond(state: &ServerState, request: Value) -> Option<Value> {
    // A missing id marks a notification; an explicit null id is still a request.
    let id = request.get("id")?.clone();
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let result = match method {
        "initialize" => Ok(initialize(&request)),
        "ping" => Ok(json!({})),
        "tools/list" => listed(&state.app).map(|tools| json!({ "tools": tools })),
        "tools/call" => call_tool(state, &request).await,
        "" => Err((-32600, "缺少方法名".into())),
        other => Err((-32601, format!("不支持的方法 {other}"))),
    };
    Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => rpc_error(id, code, message),
    })
}

/// Keep the protocol version the client asked for; a client that supports it
/// negotiated it before sending `initialize`.
fn initialize(request: &Value) -> Value {
    let version = request
        .pointer("/params/protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or("2025-06-18");
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": {
            "name": "xjtu-launcher",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn listed(app: &AppHandle) -> Result<Vec<Value>, (i64, String)> {
    let config = read_config(app)?;
    Ok(open_tools(&config.mcp_server_disabled_tools)
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": tool.parameters
            })
        })
        .collect())
}

fn read_config(app: &AppHandle) -> Result<config::AppConfig, (i64, String)> {
    let path = config_path(app).map_err(|error| (-32603, error))?;
    config::load(&path).map_err(|error| (-32603, error))
}

fn open_tools(disabled: &[String]) -> Vec<tools::ToolDef> {
    tools::builtin()
        .into_iter()
        .filter(|tool| !disabled.contains(&tool.name))
        .collect()
}

async fn call_tool(state: &ServerState, request: &Value) -> Result<Value, (i64, String)> {
    let name = request
        .pointer("/params/name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if name.is_empty() {
        return Err((-32602, "缺少工具名称".into()));
    }
    let arguments = request
        .pointer("/params/arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if !arguments.is_object() {
        return Err((-32602, "工具参数必须是对象".into()));
    }
    let Some(tool) = open_tools(&read_config(&state.app)?.mcp_server_disabled_tools)
        .into_iter()
        .find(|tool| tool.name == name)
    else {
        return Err((-32602, format!("工具不存在或未开放：{name}")));
    };
    // A failed query is a tool-level error the agent can read and retry, not a
    // protocol error: the call itself was well formed.
    Ok(match tool.call(&state.app, arguments).await {
        Ok(text) => json!({
            "content": [{ "type": "text", "text": text }],
            "isError": false
        }),
        Err(message) => json!({
            "content": [{ "type": "text", "text": message }],
            "isError": true
        }),
    })
}

fn rpc_error(id: Value, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

fn reply(value: Value) -> Reply {
    let body = serde_json::to_vec(&value).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::from_static(b"{}"))))
}

fn text(status: StatusCode, message: &'static str) -> Reply {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from_static(message.as_bytes())))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::from_static(b""))))
}

fn accepted() -> Reply {
    Response::builder()
        .status(StatusCode::ACCEPTED)
        .body(Full::new(Bytes::new()))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::new())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_negotiated_protocol_version() {
        let answer = initialize(&json!({"params": {"protocolVersion": "2025-03-26"}}));
        assert_eq!(answer["protocolVersion"], "2025-03-26");
        assert!(answer["capabilities"]["tools"].is_object());
    }

    #[test]
    fn an_occupied_port_is_refused_and_freed_on_release() {
        let occupied = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = occupied.local_addr().unwrap().port();
        let error = bind(port).unwrap_err();
        assert!(error.contains(&port.to_string()), "{error}");
        drop(occupied);
        // Whoever held the port released it, so the requested port binds again.
        bind(port).unwrap();
    }

    #[test]
    fn open_tools_skip_disabled_names() {
        let all = tools::builtin().len();
        let open = open_tools(&["campus_card".into(), "not_a_tool".into()]);
        assert_eq!(open.len(), all - 1);
        assert!(open.iter().all(|tool| tool.name != "campus_card"));
    }

    #[test]
    fn tool_list_has_name_description_and_schema() {
        for tool in tools() {
            assert!(!tool.name.is_empty());
            assert!(!tool.description.is_empty());
            assert!(!tool.category.is_empty());
        }
        let listed = open_tools(&[])
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "inputSchema": tool.parameters
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(listed.len(), tools().len());
        assert!(listed.iter().all(|item| {
            item["inputSchema"]["type"] == "object" && item["description"].is_string()
        }));
    }
}
