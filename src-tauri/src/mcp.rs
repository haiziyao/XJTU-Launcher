//! MCP server profiles and a minimal JSON-RPC client for stdio and Streamable HTTP.
//! Profiles live in ~/.xjtu-launcher/mcp.json; campus credentials never reach this module.
use crate::{config, plugins};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    process::Stdio,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::{oneshot, Mutex},
};

const PROTOCOL_VERSION: &str = "2025-06-18";
const QUICK_TIMEOUT: Duration = Duration::from_secs(20);
const CALL_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Transport {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
    },
    Http {
        url: String,
        #[serde(default)]
        headers: BTreeMap<String, String>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Server {
    pub id: String,
    pub name: String,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    pub transport: Transport,
}

fn enabled_by_default() -> bool {
    true
}

impl Server {
    pub fn validate(&self) -> Result<(), String> {
        if !plugins::valid_id(&self.id) {
            return Err("MCP 标识无效".into());
        }
        if self.name.trim().is_empty() || self.name.chars().count() > 40 {
            return Err("MCP 名称无效".into());
        }
        match &self.transport {
            Transport::Stdio { command, args, env } => {
                if command.trim().is_empty() || command.len() > 500 || command.contains('\0') {
                    return Err("启动命令无效".into());
                }
                if args.len() > 32 || args.iter().any(|a| a.len() > 500 || a.contains('\0')) {
                    return Err("启动参数无效".into());
                }
                if env.len() > 32
                    || env
                        .iter()
                        .any(|(k, v)| !valid_env_key(k) || !valid_secret(v))
                {
                    return Err("环境变量无效".into());
                }
            }
            Transport::Http { url, headers } => {
                validate_http(url)?;
                if headers.len() > 16
                    || headers.iter().any(|(k, v)| {
                        k.is_empty()
                            || k.len() > 100
                            || !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                            || !valid_secret(v)
                    })
                {
                    return Err("请求头无效".into());
                }
            }
        }
        Ok(())
    }

    /// Human-readable target; never includes environment values or header values.
    pub fn target(&self) -> String {
        match &self.transport {
            Transport::Stdio { command, args, .. } => std::iter::once(command.as_str())
                .chain(args.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join(" "),
            Transport::Http { url, .. } => url.clone(),
        }
    }
}

fn valid_env_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 100
        && !key.as_bytes()[0].is_ascii_digit()
        && key
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

fn valid_secret(value: &str) -> bool {
    value.len() <= 4096 && !value.contains('\0') && !value.contains('\n')
}

fn validate_http(text: &str) -> Result<(), String> {
    let url = plugins::validate_url(text)?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if url.scheme() != "https" && !(url.scheme() == "http" && local) {
        return Err("MCP 地址请使用 HTTPS；本机调试可使用 HTTP".into());
    }
    if url.fragment().is_some() {
        return Err("MCP 地址不能包含片段".into());
    }
    Ok(())
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Book {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub servers: Vec<Server>,
}

fn default_version() -> u32 {
    1
}

impl Default for Book {
    fn default() -> Self {
        Self {
            version: 1,
            servers: Vec::new(),
        }
    }
}

impl Book {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.servers.len() > 64 {
            return Err("MCP 配置版本不受支持或项目过多".into());
        }
        let mut ids = std::collections::HashSet::new();
        for server in &self.servers {
            server.validate()?;
            if !ids.insert(&server.id) {
                return Err("MCP 配置包含重复项目".into());
            }
        }
        Ok(())
    }
}

pub fn load(path: &Path) -> Result<Book, String> {
    if !path.exists() {
        return Ok(Book::default());
    }
    let bytes = std::fs::read(path).map_err(|_| "无法读取 MCP 配置".to_string())?;
    let book: Book = serde_json::from_slice(&bytes)
        .map_err(|_| "MCP 配置损坏，已保留原文件，请修复后重试".to_string())?;
    book.validate()?;
    Ok(book)
}

pub fn save(path: &Path, book: &Book) -> Result<(), String> {
    book.validate()?;
    config::write_atomic(
        path,
        &serde_json::to_vec_pretty(book).map_err(|_| "无法序列化 MCP 配置")?,
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicServer {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub kind: String,
    pub target: String,
    pub secrets: Vec<String>,
    /// Same shape as the stored transport, with every secret value blanked for editing.
    pub transport: Transport,
}

pub fn public(book: &Book) -> Vec<PublicServer> {
    book.servers
        .iter()
        .map(|server| {
            let (kind, secrets, transport) = match &server.transport {
                Transport::Stdio { command, args, env } => (
                    "stdio",
                    env.keys().cloned().collect(),
                    Transport::Stdio {
                        command: command.clone(),
                        args: args.clone(),
                        env: env.keys().map(|key| (key.clone(), String::new())).collect(),
                    },
                ),
                Transport::Http { url, headers } => (
                    "http",
                    headers.keys().cloned().collect::<Vec<_>>(),
                    Transport::Http {
                        url: url.clone(),
                        headers: headers
                            .keys()
                            .map(|key| (key.clone(), String::new()))
                            .collect(),
                    },
                ),
            };
            PublicServer {
                id: server.id.clone(),
                name: server.name.clone(),
                enabled: server.enabled,
                kind: kind.into(),
                target: server.target(),
                secrets,
                transport,
            }
        })
        .collect()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub id: Option<String>,
    pub name: String,
    pub transport: Transport,
}

/// Adds a server or replaces the one with the same id. Blank values keep the stored secret.
pub fn upsert(book: &mut Book, input: Input) -> Result<String, String> {
    let id = match input.id {
        Some(id) => id,
        None => {
            let mut base: String = input
                .name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect();
            while base.starts_with('-') || base.starts_with('.') {
                base.remove(0);
            }
            base.truncate(60);
            if base.len() < 3 {
                base.push_str("-mcp");
            }
            let mut candidate = base.clone();
            let mut suffix = 2;
            while book.servers.iter().any(|s| s.id == candidate) {
                candidate = format!("{base}-{suffix}");
                suffix += 1;
            }
            candidate
        }
    };
    let previous = book.servers.iter().position(|s| s.id == id);
    let mut transport = input.transport;
    if let Some(index) = previous {
        merge_secrets(&mut transport, &book.servers[index].transport);
    }
    let server = Server {
        id: id.clone(),
        name: input.name,
        enabled: previous
            .map(|index| book.servers[index].enabled)
            .unwrap_or(true),
        transport,
    };
    server.validate()?;
    match previous {
        Some(index) => book.servers[index] = server,
        None => book.servers.push(server),
    }
    Ok(id)
}

fn merge_secrets(next: &mut Transport, previous: &Transport) {
    match (next, previous) {
        (Transport::Stdio { env, .. }, Transport::Stdio { env: old_env, .. }) => {
            for (key, value) in env.iter_mut() {
                if value.is_empty() {
                    if let Some(kept) = old_env.get(key) {
                        *value = kept.clone();
                    }
                }
            }
        }
        (Transport::Http { headers, .. }, Transport::Http { headers: old, .. }) => {
            for (key, value) in headers.iter_mut() {
                if value.is_empty() {
                    if let Some(kept) = old.get(key) {
                        *value = kept.clone();
                    }
                }
            }
        }
        _ => {}
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

struct Pending {
    waiters: Mutex<HashMap<u64, oneshot::Sender<Value>>>,
}

enum Wire {
    Stdio {
        // Kept for its kill_on_drop side effect; the child dies with the session.
        _child: Child,
        stdin: ChildStdin,
        pending: Arc<Pending>,
    },
    Http {
        client: reqwest::Client,
        url: url::Url,
        headers: reqwest::header::HeaderMap,
        session: Option<String>,
    },
}

pub struct Session {
    wire: Wire,
    next: AtomicU64,
}

impl Session {
    pub async fn connect(server: &Server) -> Result<Self, String> {
        server.validate()?;
        let wire = match &server.transport {
            Transport::Stdio { command, args, env } => {
                let mut process = Command::new(command);
                process
                    .args(args)
                    .envs(env)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .kill_on_drop(true);
                let mut child = process
                    .spawn()
                    .map_err(|_| format!("无法启动 MCP 服务器：{command}"))?;
                let stdin = child.stdin.take().ok_or("无法连接 MCP 服务器")?;
                let stdout = child.stdout.take().ok_or("无法连接 MCP 服务器")?;
                let pending = Arc::new(Pending {
                    waiters: Mutex::new(HashMap::new()),
                });
                read_stdio(stdout, pending.clone());
                Wire::Stdio {
                    _child: child,
                    stdin,
                    pending,
                }
            }
            Transport::Http { url, headers } => {
                let url = url::Url::parse(url).map_err(|_| "MCP 地址无效")?;
                let mut map = reqwest::header::HeaderMap::new();
                for (key, value) in headers {
                    let name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                        .map_err(|_| "请求头无效")?;
                    let value =
                        reqwest::header::HeaderValue::from_str(value).map_err(|_| "请求头无效")?;
                    map.insert(name, value);
                }
                let client = reqwest::Client::builder()
                    .timeout(CALL_TIMEOUT)
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                    .map_err(|_| "无法初始化 MCP 连接")?;
                Wire::Http {
                    client,
                    url,
                    headers: map,
                    session: None,
                }
            }
        };
        let mut session = Session {
            wire,
            next: AtomicU64::new(0),
        };
        let result = session
            .request(
                "initialize",
                json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": { "name": "XJTU Launcher", "version": env!("CARGO_PKG_VERSION") }
                }),
                QUICK_TIMEOUT,
            )
            .await?;
        if !result.is_object() {
            return Err("MCP 服务器初始化响应无效".into());
        }
        session.notify("notifications/initialized").await?;
        Ok(session)
    }

    pub async fn list_tools(&mut self) -> Result<Vec<Tool>, String> {
        let mut tools = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..10 {
            let mut params = json!({});
            if let Some(cursor) = &cursor {
                params["cursor"] = json!(cursor);
            }
            let result = self.request("tools/list", params, QUICK_TIMEOUT).await?;
            let list = result
                .get("tools")
                .and_then(Value::as_array)
                .ok_or("MCP 工具列表格式无效")?;
            for item in list {
                let Some(name) = item.get("name").and_then(Value::as_str) else {
                    continue;
                };
                if name.is_empty() {
                    continue;
                }
                tools.push(Tool {
                    name: name.to_string(),
                    description: item
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    input_schema: item
                        .get("inputSchema")
                        .cloned()
                        .unwrap_or_else(|| json!({"type": "object"})),
                });
            }
            cursor = result
                .get("nextCursor")
                .and_then(Value::as_str)
                .map(str::to_string);
            if cursor.is_none() {
                break;
            }
        }
        Ok(tools)
    }

    /// Used by the Agent tool loop; the management page only probes tool lists.
    #[allow(dead_code)]
    pub async fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, String> {
        self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
            CALL_TIMEOUT,
        )
        .await
    }

    async fn request(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        match &mut self.wire {
            Wire::Stdio { stdin, pending, .. } => {
                let (sender, receiver) = oneshot::channel();
                pending.waiters.lock().await.insert(id, sender);
                let line = serde_json::to_vec(&json!({
                    "jsonrpc": "2.0", "id": id, "method": method, "params": params
                }))
                .map_err(|_| "无法编码 MCP 请求")?;
                if let Err(error) = write_line(stdin, &line).await {
                    pending.waiters.lock().await.remove(&id);
                    return Err(error);
                }
                let response = tokio::time::timeout(timeout, receiver).await;
                pending.waiters.lock().await.remove(&id);
                match response {
                    Ok(Ok(message)) => response_value(message),
                    Ok(Err(_)) => Err("MCP 服务器已断开".into()),
                    Err(_) => Err("MCP 服务器响应超时".into()),
                }
            }
            Wire::Http {
                client,
                url,
                headers,
                session,
            } => {
                let payload = json!({
                    "jsonrpc": "2.0", "id": id, "method": method, "params": params
                });
                let body = post(client, url, headers, session, &payload).await?;
                let body = body.ok_or("MCP 服务器未返回响应")?;
                for message in decode_body(&body) {
                    if message.get("id").and_then(Value::as_u64) == Some(id) {
                        return response_value(message);
                    }
                }
                Err("MCP 服务器响应格式无效".into())
            }
        }
    }

    async fn notify(&mut self, method: &str) -> Result<(), String> {
        match &mut self.wire {
            Wire::Stdio { stdin, .. } => {
                let line = serde_json::to_vec(&json!({ "jsonrpc": "2.0", "method": method }))
                    .map_err(|_| "无法编码 MCP 请求")?;
                write_line(stdin, &line).await
            }
            Wire::Http {
                client,
                url,
                headers,
                session,
            } => post(
                client,
                url,
                headers,
                session,
                &json!({ "jsonrpc": "2.0", "method": method }),
            )
            .await
            .map(|_| ()),
        }
    }
}

fn read_stdio(stdout: ChildStdout, pending: Arc<Pending>) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let Ok(message) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let Some(id) = message.get("id").and_then(Value::as_u64) else {
                // Server-initiated requests are not supported; notifications need no answer.
                continue;
            };
            if message.get("result").is_none() && message.get("error").is_none() {
                continue;
            }
            if let Some(sender) = pending.waiters.lock().await.remove(&id) {
                let _ = sender.send(message);
            }
        }
    });
}

async fn write_line(stdin: &mut ChildStdin, line: &[u8]) -> Result<(), String> {
    let write = async {
        stdin
            .write_all(line)
            .await
            .map_err(|_| "无法写入 MCP 服务器")?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|_| "无法写入 MCP 服务器")?;
        stdin
            .flush()
            .await
            .map_err(|_| "无法写入 MCP 服务器".to_string())
    };
    tokio::time::timeout(Duration::from_secs(5), write)
        .await
        .map_err(|_| "写入 MCP 服务器超时")??;
    Ok(())
}

async fn post(
    client: &reqwest::Client,
    url: &url::Url,
    headers: &reqwest::header::HeaderMap,
    session: &mut Option<String>,
    payload: &Value,
) -> Result<Option<String>, String> {
    let mut request = client
        .post(url.clone())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(
            reqwest::header::ACCEPT,
            "application/json, text/event-stream",
        )
        .header("MCP-Protocol-Version", PROTOCOL_VERSION)
        .headers(headers.clone())
        .json(payload);
    if let Some(id) = session.as_deref() {
        request = request.header("Mcp-Session-Id", id);
    }
    let response = request.send().await.map_err(|_| "无法连接 MCP 服务器")?;
    if response.status() == reqwest::StatusCode::ACCEPTED {
        return Ok(None);
    }
    if response.status() == reqwest::StatusCode::UNAUTHORIZED
        || response.status() == reqwest::StatusCode::FORBIDDEN
    {
        return Err("MCP 服务器需要认证，请检查令牌".into());
    }
    if !response.status().is_success() {
        return Err(format!(
            "MCP 服务器返回 HTTP {}",
            response.status().as_u16()
        ));
    }
    if session.is_none() {
        if let Some(id) = response.headers().get("mcp-session-id") {
            *session = id.to_str().ok().map(str::to_string);
        }
    }
    response
        .text()
        .await
        .map(Some)
        .map_err(|_| "无法读取 MCP 响应".to_string())
}

fn decode_body(body: &str) -> Vec<Value> {
    let trimmed = body.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return match serde_json::from_str::<Value>(trimmed) {
            Ok(Value::Array(items)) => items,
            Ok(other) => vec![other],
            Err(_) => Vec::new(),
        };
    }
    let mut messages = Vec::new();
    let mut data = String::new();
    for line in body.lines() {
        if line.is_empty() {
            if !data.is_empty() {
                if let Ok(value) = serde_json::from_str(&data) {
                    messages.push(value);
                }
                data.clear();
            }
        } else if let Some(rest) = line.strip_prefix("data:") {
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest);
        }
    }
    if !data.is_empty() {
        if let Ok(value) = serde_json::from_str(&data) {
            messages.push(value);
        }
    }
    messages
}

fn response_value(message: Value) -> Result<Value, String> {
    if let Some(error) = message.get("error") {
        let text = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("MCP 服务器返回错误");
        return Err(text.to_string());
    }
    Ok(message.get("result").cloned().unwrap_or(Value::Null))
}

/// Connects, lists tools and closes the session in one go.
pub async fn probe(server: &Server) -> Result<Vec<Tool>, String> {
    let mut session = Session::connect(server).await?;
    session.list_tools().await
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Field {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub secret: bool,
    #[serde(default)]
    pub placeholder: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub transport: Transport,
    #[serde(default)]
    pub fields: Vec<Field>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Catalog {
    pub schema_version: u32,
    #[serde(default)]
    pub servers: Vec<Entry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteCatalog {
    pub source: Option<String>,
    pub default_source: Option<String>,
    pub servers: Vec<Entry>,
}

/// Marketplace entries served from the plugin server's `mcp.json`.
pub async fn catalog(config: &config::AppConfig) -> Result<RemoteCatalog, String> {
    let source = crate::market::source(config)?;
    let mut result = RemoteCatalog {
        source: source.clone(),
        default_source: crate::market::default_server().map(str::to_string),
        servers: Vec::new(),
    };
    if let Some(source) = source {
        let url = crate::market::source_url(&source)?;
        let data = crate::market::fetch(
            url.join("mcp.json").map_err(|_| "MCP 目录地址无效")?,
            256 * 1024,
            |_, _| {},
        )
        .await?;
        let catalog: Catalog = serde_json::from_slice(&data).map_err(|_| "MCP 目录格式无效")?;
        validate_catalog(&catalog)?;
        result.servers = catalog.servers;
    }
    Ok(result)
}

pub fn validate_catalog(catalog: &Catalog) -> Result<(), String> {
    if catalog.schema_version != 1 || catalog.servers.len() > 64 {
        return Err("MCP 目录版本不受支持或项目过多".into());
    }
    let mut ids = std::collections::HashSet::new();
    for entry in &catalog.servers {
        if !ids.insert(&entry.id)
            || entry.name.trim().is_empty()
            || entry.name.chars().count() > 40
            || entry.description.chars().count() > 300
        {
            return Err("MCP 目录包含无效或重复项目".into());
        }
        Server {
            id: entry.id.clone(),
            name: entry.name.clone(),
            enabled: true,
            transport: entry.transport.clone(),
        }
        .validate()?;
        if entry.fields.len() > 16 {
            return Err("MCP 目录凭据字段过多".into());
        }
    }
    Ok(())
}

/// Turns a catalog entry and the values the user typed into a local server profile.
pub fn from_entry(entry: &Entry, values: &BTreeMap<String, String>) -> Result<Server, String> {
    let mut transport = entry.transport.clone();
    for field in &entry.fields {
        let value = values.get(&field.key).cloned().unwrap_or_default();
        if value.len() > 4096 || value.contains('\0') {
            return Err(format!("{} 的取值无效", field.label));
        }
        match &mut transport {
            Transport::Stdio { env, .. } => {
                env.insert(field.key.clone(), value);
            }
            Transport::Http { headers, .. } => {
                if value.is_empty() {
                    continue;
                }
                headers.insert(field.key.clone(), value);
            }
        }
    }
    let server = Server {
        id: entry.id.clone(),
        name: entry.name.clone(),
        enabled: true,
        transport,
    };
    server.validate()?;
    Ok(server)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stdio(id: &str) -> Server {
        Server {
            id: id.into(),
            name: "Files".into(),
            enabled: true,
            transport: Transport::Stdio {
                command: "npx".into(),
                args: vec![
                    "-y".into(),
                    "@modelcontextprotocol/server-filesystem".into(),
                ],
                env: BTreeMap::from([("TOKEN".to_string(), "secret".to_string())]),
            },
        }
    }

    #[test]
    fn validates_transports_and_public_view_hides_secrets() {
        assert!(stdio("files").validate().is_ok());
        assert!(stdio("../escape").validate().is_err());
        let mut book = Book::default();
        book.servers.push(stdio("files"));
        let view = public(&book);
        assert_eq!(
            view[0].target,
            "npx -y @modelcontextprotocol/server-filesystem"
        );
        assert_eq!(view[0].secrets, vec!["TOKEN".to_string()]);
        assert!(!format!("{:?}", view[0].secrets).contains("secret"));
        let http = Server {
            id: "remote".into(),
            name: "Remote".into(),
            enabled: true,
            transport: Transport::Http {
                url: "http://remote.test/mcp".into(),
                headers: BTreeMap::new(),
            },
        };
        assert!(http.validate().is_err());
    }

    #[test]
    fn keeps_stored_secret_when_edit_leaves_it_blank() {
        let mut book = Book::default();
        book.servers.push(stdio("files"));
        let id = upsert(
            &mut book,
            Input {
                id: Some("files".into()),
                name: "Files".into(),
                transport: Transport::Stdio {
                    command: "npx".into(),
                    args: vec![],
                    env: BTreeMap::from([("TOKEN".to_string(), String::new())]),
                },
            },
        )
        .unwrap();
        assert_eq!(id, "files");
        match &book.servers[0].transport {
            Transport::Stdio { env, .. } => assert_eq!(env["TOKEN"], "secret"),
            _ => panic!("wrong transport"),
        }
    }

    #[test]
    fn builds_profile_from_catalog_entry() {
        let entry = Entry {
            id: "github".into(),
            name: "GitHub".into(),
            description: "Issues and pull requests".into(),
            transport: Transport::Stdio {
                command: "npx".into(),
                args: vec!["-y".into(), "server-github".into()],
                env: BTreeMap::new(),
            },
            fields: vec![Field {
                key: "GITHUB_TOKEN".into(),
                label: "GitHub 令牌".into(),
                secret: true,
                placeholder: "ghp_…".into(),
            }],
        };
        let server = from_entry(
            &entry,
            &BTreeMap::from([("GITHUB_TOKEN".to_string(), "token".to_string())]),
        )
        .unwrap();
        match &server.transport {
            Transport::Stdio { env, .. } => assert_eq!(env["GITHUB_TOKEN"], "token"),
            _ => panic!("wrong transport"),
        }
        assert!(validate_catalog(&Catalog {
            schema_version: 1,
            servers: vec![entry.clone(), entry],
        })
        .is_err());
    }

    #[test]
    fn parses_sse_and_plain_json_bodies() {
        let plain = decode_body("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}");
        assert_eq!(plain.len(), 1);
        let sse =
            decode_body("event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}\n\n");
        assert_eq!(sse.len(), 1);
        assert_eq!(sse[0]["id"], 2);
    }

    #[tokio::test]
    #[ignore = "spawns node; run manually to verify the stdio transport"]
    async fn stdio_session_lists_and_calls_tools() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("server.mjs");
        std::fs::write(
            &script,
            r#"
import { createInterface } from 'node:readline';
const rl = createInterface({ input: process.stdin });
rl.on('line', (line) => {
  const message = JSON.parse(line);
  if (message.id === undefined) return;
  const reply = (result) =>
    process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id: message.id, result }) + '\n');
  if (message.method === 'initialize')
    reply({ protocolVersion: '2025-06-18', capabilities: { tools: {} }, serverInfo: { name: 'mock', version: '1' } });
  else if (message.method === 'tools/list')
    reply({ tools: [{ name: 'echo', description: 'Echo', inputSchema: { type: 'object' } }] });
  else if (message.method === 'tools/call')
    reply({ content: [{ type: 'text', text: message.params.arguments.text }] });
  else reply({});
});
"#,
        )
        .unwrap();
        let server = Server {
            id: "mock".into(),
            name: "Mock".into(),
            enabled: true,
            transport: Transport::Stdio {
                command: "node".into(),
                args: vec![script.to_string_lossy().into_owned()],
                env: BTreeMap::new(),
            },
        };
        let mut session = Session::connect(&server).await.unwrap();
        let tools = session.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
        let result = session
            .call_tool("echo", json!({ "text": "hello" }))
            .await
            .unwrap();
        assert_eq!(result["content"][0]["text"], "hello");
    }
}
