//! Loopback MCP endpoint for the personal page.
//!
//! This is the opposite direction from `mcp.rs`: instead of connecting to
//! external MCP servers, it offers the launcher's campus tools to coding
//! agents. There is no request authentication by user decision, so the listener
//! is bound to 127.0.0.1 only. Most tools are queries; the write tools (homework
//! upload, evaluation submission, local timetable edits) only act after an
//! explicit `confirm=true`, so a mistaken call cannot submit or overwrite.
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
    /// The port actually serving, which can differ from the saved one after a
    /// fallback to a free port.
    pub port: u16,
    pub address: String,
    pub error: Option<String>,
}

#[derive(serde::Serialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    /// Short Chinese name the page shows instead of the raw tool name.
    pub title: String,
    /// One line saying what the tool does; no argument chatter.
    pub summary: String,
    /// Section in the page's tool list.
    pub category: String,
    /// What the tool does to data: `read`, `write`, `update` or `delete`. The
    /// page marks each kind so a tool that changes something stands out.
    pub access: Vec<&'static str>,
}

pub fn address(port: u16) -> String {
    format!("http://127.0.0.1:{port}{ENDPOINT}")
}

impl State {
    /// The address reflects the listener that is actually bound, so a port
    /// change is not claimed before the new socket is open.
    pub fn status(&self, enabled: bool, configured_port: u16) -> Status {
        let live = self.running_port();
        let port = live.unwrap_or(configured_port);
        Status {
            running: live.is_some(),
            enabled,
            port,
            address: address(port),
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

/// Loopback bind shared by the startup and port-change paths. The preferred
/// port is tried first; if another process holds it, the OS picks a free one
/// instead of leaving the user with a switch that cannot be turned on. The
/// returned port is the one actually bound.
fn bind(preferred: u16) -> Result<(TcpListener, u16), String> {
    if let Ok(listener) = TcpListener::bind(("127.0.0.1", preferred)) {
        return Ok((listener, preferred));
    }
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr().map(|addr| (listener, addr.port())))
        .map_err(|error| format!("无法监听本机端口：{error}"))
}

/// Bind and serve on the calling thread's runtime. The caller learns which port
/// is serving so the page can show the real address after a fallback.
pub fn start(app: &AppHandle, state: &State, preferred: u16) -> Result<u16, String> {
    let mut running = state.running.lock().map_err(|_| "MCP 服务状态不可用")?;
    if let Some(current) = running.as_ref() {
        if current.port == preferred {
            return Ok(current.port);
        }
        let message = format!("MCP 服务已在端口 {} 运行", current.port);
        state.set_error(Some(message.clone()));
        return Err(message);
    }
    let (listener, port) = match bind(preferred) {
        Ok(bound) => bound,
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
    Ok(port)
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
    let mut list: Vec<Tool> = tools::builtin()
        .into_iter()
        .map(|tool| {
            let (category, title, summary) = label(&tool.name);
            Tool {
                access: access(&tool.name).to_vec(),
                category: category.into(),
                title: title.into(),
                summary: summary.into(),
                name: tool.name,
                description: tool.description,
            }
        })
        .collect();
    // Sorting by section keeps each group together while the stable sort keeps
    // the tool order inside it.
    list.sort_by_key(|tool| group_rank(&tool.category));
    list
}

/// Every tool reads campus data; only a few change something. Those are the
/// ones a user should never mistake for a query.
fn access(name: &str) -> &'static [&'static str] {
    match name {
        // save creates or changes a course, hide changes, reset deletes.
        "campus_timetable_edit" => &["write", "update", "delete"],
        // Uploading and submitting homework changes the course's state.
        "campus_lms_submit" => &["write"],
        "campus_evaluations_submit" => &["write"],
        // The workbench tool copies, renames and deletes the user's files.
        "workbench_edit" => &["write", "update", "delete"],
        _ => &["read"],
    }
}

/// Display name, one-line purpose and section for each built-in tool. Kept in
/// the backend so the page and `tools/list` describe the same tools.
fn label(name: &str) -> (&'static str, &'static str, &'static str) {
    match name {
        "campus_timetable" => ("课程与成绩", "课表查询", "查看本学期课表"),
        "campus_timetable_edit" => ("课程与成绩", "修改课表", "新增、隐藏或恢复本机课程"),
        "campus_academic" => ("课程与成绩", "学业查询", "查看成绩、考试、校历与学籍"),
        "campus_classroom" => ("课程与成绩", "教室查询", "查询空闲自习教室"),
        "campus_lms" => ("学习", "思源学堂", "查看课程、活动、作业与课程资料"),
        "campus_lms_download" => ("学习", "下载课程资料", "把课程资料附件保存到本机下载文件夹"),
        "campus_lms_submit" => ("学习", "提交作业", "上传并提交思源学堂作业"),
        "campus_evaluation" => ("学习", "评教", "查看待评问卷并生成预览"),
        "campus_evaluations_submit" => ("学习", "提交评教", "提交已确认的评教"),
        "campus_attendance" => ("校园生活", "考勤", "查询考勤记录"),
        "campus_card" => ("校园生活", "校园卡", "查询余额与消费流水"),
        "campus_fitness" => ("校园生活", "体测", "查询体测成绩"),
        "campus_library" => ("校园生活", "图书馆", "查询楼层、区域与空闲座位"),
        "campus_venues" => ("校园生活", "场馆", "查看可预约场馆与时段"),
        "campus_notices" => ("通知", "校园通知", "查看教务处等栏目公告"),
        "workbench_files" => ("工作台", "查看文件", "浏览工作台文件夹、文件与搜索"),
        "workbench_edit" => ("工作台", "编辑文件", "导入、建文件夹、绑定或删除文件"),
        _ => ("其他", "其他", ""),
    }
}

/// Fixed section order for the page; unknown sections land at the end.
fn group_rank(category: &str) -> usize {
    match category {
        "课程与成绩" => 0,
        "学习" => 1,
        "工作台" => 2,
        "校园生活" => 3,
        "通知" => 4,
        _ => 5,
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
    let requested = request
        .pointer("/params/name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if requested.is_empty() {
        return Err((-32602, "缺少工具名称".into()));
    }
    let arguments = request
        .pointer("/params/arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if !arguments.is_object() {
        return Err((-32602, "工具参数必须是对象".into()));
    }
    // A saved pre-merge name is translated before the exposure check, so the
    // merged tool's on/off setting applies to both names.
    let (name, arguments) = match tools::legacy(requested, &arguments) {
        Some((tool, args)) => (tool, args),
        None => (requested, arguments),
    };
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
    fn an_occupied_port_falls_back_to_a_free_one() {
        let occupied = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let taken = occupied.local_addr().unwrap().port();
        let (_listener, port) = bind(taken).unwrap();
        assert_ne!(port, taken);
        assert!(port > 0);
        drop(occupied);
        // Once the preferred port is free it is used again.
        assert_eq!(bind(taken).unwrap().1, taken);
    }

    #[test]
    fn writing_tools_are_marked_apart_from_queries() {
        assert_eq!(access("campus_timetable"), &["read"]);
        assert!(access("campus_timetable_edit").contains(&"delete"));
        assert_eq!(access("campus_lms_submit"), &["write"]);
        assert!(access("workbench_edit").contains(&"delete"));
        assert_eq!(access("workbench_files"), &["read"]);
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
            assert!(!tool.access.is_empty());
            assert_ne!(tool.title, "其他", "{} 缺少显示名称", tool.name);
            assert!(!tool.summary.is_empty());
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
