//! Tools the Agent can call: campus queries backed by the school adapters and
//! tools exposed by enabled MCP servers.
use crate::{mcp, school};
use serde_json::{json, Value};
use tauri::AppHandle;

/// Tool output above this size is truncated before it reaches the model.
const OUTPUT_LIMIT: usize = 24_000;

#[derive(Clone)]
pub enum Kind {
    Campus(&'static str),
    Mcp { server: mcp::Server, tool: String },
}

#[derive(Clone)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub kind: Kind,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub source: String,
}

fn schema(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
}

fn campus(
    name: &'static str,
    description: &'static str,
    properties: Value,
    required: &[&str],
) -> ToolDef {
    ToolDef {
        name: name.into(),
        description: description.into(),
        parameters: schema(properties, required),
        kind: Kind::Campus(name),
    }
}

pub fn builtin() -> Vec<ToolDef> {
    let term =
        json!({ "type": "string", "description": "学期，例如 2025-2026-1；留空使用当前学期" });
    let page = json!({ "type": "integer", "minimum": 1, "maximum": 500, "description": "页码，从 1 开始" });
    let date = json!({ "type": "string", "description": "日期，格式 YYYY-MM-DD" });
    vec![
        campus("campus_timetable", "查询当前学期的课程表", json!({}), &[]),
        campus("campus_terms", "列出可选学期", json!({}), &[]),
        campus(
            "campus_grades",
            "查询本科成绩",
            json!({ "term": term }),
            &[],
        ),
        campus("campus_exams", "查询考试安排", json!({ "term": term }), &[]),
        campus(
            "campus_calendar",
            "查询校历图片地址和学年信息",
            json!({}),
            &[],
        ),
        campus(
            "campus_attendance_streams",
            "查询考勤流水",
            json!({ "start": date, "end": date, "page": page }),
            &[],
        ),
        campus(
            "campus_attendance_records",
            "按日期范围查询课程考勤",
            json!({ "start": date, "end": date, "page": page }),
            &["start", "end"],
        ),
        campus("campus_card", "查询校园卡余额", json!({}), &[]),
        campus(
            "campus_card_transactions",
            "查询校园卡流水",
            json!({ "start": date, "end": date, "page": page }),
            &["start", "end"],
        ),
        campus("campus_fitness", "查询体测概览", json!({}), &[]),
        campus(
            "campus_fitness_score",
            "查询某一学年的体测成绩",
            json!({ "year": json!({ "type": "string", "description": "学年，例如 2024-2025" }) }),
            &["year"],
        ),
        campus(
            "campus_room_options",
            "列出自习教室查询可用的校区和楼宇",
            json!({}),
            &[],
        ),
        campus(
            "campus_rooms",
            "查询空闲教室",
            json!({
                "campus": { "type": "string" },
                "building": { "type": "string" },
                "date": date,
                "start": { "type": "integer", "minimum": 1, "maximum": 14 },
                "end": { "type": "integer", "minimum": 1, "maximum": 14 }
            }),
            &["campus", "building", "date", "start", "end"],
        ),
        campus(
            "campus_courses",
            "按名称查询全校课程",
            json!({
                "term": term,
                "name": { "type": "string" },
                "campus": { "type": "string" }
            }),
            &["name", "campus"],
        ),
    ]
}

fn text(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn number(args: &Value, key: &str, fallback: u32) -> u32 {
    args.get(key)
        .and_then(Value::as_u64)
        .map(|value| value as u32)
        .unwrap_or(fallback)
}

fn required(args: &Value, key: &str) -> Result<String, String> {
    text(args, key).ok_or_else(|| format!("缺少参数 {key}"))
}

fn query_for(name: &str, args: &Value) -> Result<school::Query, String> {
    let query = match name {
        "campus_terms" => school::Query::Terms,
        "campus_grades" => school::Query::Grades {
            term: text(args, "term"),
        },
        "campus_exams" => school::Query::Exams {
            term: text(args, "term"),
        },
        "campus_calendar" => school::Query::Calendar,
        "campus_attendance_streams" => school::Query::AttendanceStreams {
            start: text(args, "start"),
            end: text(args, "end"),
            page: number(args, "page", 1),
        },
        "campus_attendance_records" => school::Query::AttendanceRecords {
            start: required(args, "start")?,
            end: required(args, "end")?,
            page: number(args, "page", 1),
        },
        "campus_card" => school::Query::Card,
        "campus_card_transactions" => school::Query::CardTransactions {
            start: required(args, "start")?,
            end: required(args, "end")?,
            page: number(args, "page", 1),
        },
        "campus_fitness" => school::Query::Fitness,
        "campus_fitness_score" => school::Query::FitnessScore {
            year: required(args, "year")?,
        },
        "campus_room_options" => school::Query::RoomOptions,
        "campus_rooms" => school::Query::Rooms {
            campus: required(args, "campus")?,
            building: required(args, "building")?,
            date: required(args, "date")?,
            start: number(args, "start", 0) as u8,
            end: number(args, "end", 0) as u8,
        },
        "campus_courses" => school::Query::Courses {
            term: text(args, "term"),
            name: required(args, "name")?,
            campus: required(args, "campus")?,
        },
        other => return Err(format!("未知的校园工具 {other}")),
    };
    Ok(query)
}

fn truncate(mut text: String) -> String {
    if text.chars().count() <= OUTPUT_LIMIT {
        return text;
    }
    let cut: String = text.chars().take(OUTPUT_LIMIT).collect();
    text = format!("{cut}\n…输出过长，已截断");
    text
}

fn render_mcp(value: Value) -> String {
    if let Some(items) = value.get("content").and_then(Value::as_array) {
        let texts: Vec<String> = items
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .map(str::to_string)
            .collect();
        if !texts.is_empty() {
            return truncate(texts.join("\n"));
        }
    }
    truncate(value.to_string())
}

impl ToolDef {
    pub async fn call(&self, app: &AppHandle, args: Value) -> Result<String, String> {
        match &self.kind {
            Kind::Campus(name) => match *name {
                "campus_timetable" => {
                    let timetable = crate::run_timetable(app).await?;
                    Ok(truncate(
                        serde_json::to_string(&timetable).map_err(|_| "无法序列化课表")?,
                    ))
                }
                other => {
                    let query = query_for(other, &args)?;
                    let data = crate::run_school_query(app, query, None).await?;
                    Ok(truncate(
                        serde_json::to_string(&data).map_err(|_| "无法序列化查询结果")?,
                    ))
                }
            },
            Kind::Mcp { server, tool } => {
                let mut session = mcp::Session::connect(server).await?;
                let result = session.call_tool(tool, args).await?;
                Ok(render_mcp(result))
            }
        }
    }
}

/// Built-in tools plus the tools of every enabled MCP server.
pub async fn build(app: &AppHandle) -> Vec<ToolDef> {
    let mut defs = builtin();
    let path = match crate::mcp_path(app) {
        Ok(path) => path,
        Err(_) => return defs,
    };
    let book = match mcp::load(&path) {
        Ok(book) => book,
        Err(_) => return defs,
    };
    for server in book.servers.iter().filter(|server| server.enabled) {
        let Ok(tools) = mcp::probe(server).await else {
            // A broken server must not stop the conversation.
            continue;
        };
        for tool in tools {
            let name = mcp_tool_name(&server.id, &tool.name);
            defs.push(ToolDef {
                name,
                description: if tool.description.is_empty() {
                    format!("{} 提供的工具", server.name)
                } else {
                    tool.description
                },
                parameters: tool.input_schema,
                kind: Kind::Mcp {
                    server: server.clone(),
                    tool: tool.name,
                },
            });
        }
    }
    defs
}

fn mcp_tool_name(server: &str, tool: &str) -> String {
    let sanitize = |value: &str| -> String {
        value
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    };
    format!("mcp__{}__{}", sanitize(server), sanitize(tool))
}

pub fn info(defs: &[ToolDef]) -> Vec<ToolInfo> {
    defs.iter()
        .map(|def| ToolInfo {
            name: def.name.clone(),
            description: def.description.clone(),
            source: match def.kind {
                Kind::Campus(_) => "campus".into(),
                Kind::Mcp { .. } => "mcp".into(),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_tool_names_to_queries() {
        assert!(matches!(
            query_for("campus_grades", &json!({"term": "2025-2026-1"})).unwrap(),
            school::Query::Grades { .. }
        ));
        assert!(matches!(
            query_for("campus_card", &json!({})).unwrap(),
            school::Query::Card
        ));
        assert!(query_for("campus_attendance_records", &json!({})).is_err());
        assert!(query_for("nope", &json!({})).is_err());
    }

    #[test]
    fn sanitizes_mcp_tool_names() {
        assert_eq!(
            mcp_tool_name("cn.xjtu.campus", "read file"),
            "mcp__cn_xjtu_campus__read_file"
        );
    }

    #[test]
    fn truncates_long_output() {
        let long = "x".repeat(OUTPUT_LIMIT + 10);
        let cut = truncate(long);
        assert!(cut.contains("已截断"));
    }

    #[test]
    fn builtin_tools_are_unique() {
        let defs = builtin();
        let mut names: Vec<&str> = defs.iter().map(|def| def.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), defs.len());
        assert!(defs.iter().all(|def| def.parameters["type"] == "object"));
    }

    #[test]
    fn renders_mcp_text_content() {
        let rendered = render_mcp(json!({"content": [{"type": "text", "text": "hello"}]}));
        assert_eq!(rendered, "hello");
    }
}
