//! Tools the Agent can call: campus queries backed by the school adapters and
//! tools exposed by enabled MCP servers.
use crate::{mcp, school};
use serde_json::{json, Value};
use std::collections::BTreeMap;
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
        campus(
            "campus_evaluations",
            "列出教学评价（只读，不会提交任何评价）",
            json!({
                "term": term,
                "finished": { "type": "boolean", "description": "true 查看已评，false 查看待评" }
            }),
            &[],
        ),
        campus(
            "campus_evaluations_preview",
            "自动填写待评问卷并生成预览（不会提交）。ids 来自 campus_evaluations 的 id；rank 1 最高、5 最低；comments 可按问卷编号为每位老师单独设置文字评语。",
            json!({
                "ids": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 32,
                    "items": { "type": "string" },
                    "description": "问卷编号列表，来自 campus_evaluations"
                },
                "rank": { "type": "integer", "minimum": 1, "maximum": 5, "description": "评分档位，1 最高、5 最低，默认 2" },
                "comment": { "type": "string", "description": "默认文字评语，用于所有必填文字题" },
                "comments": {
                    "type": "object",
                    "additionalProperties": { "type": "string" },
                    "description": "按问卷编号覆盖该老师的文字评语"
                }
            }),
            &["ids"],
        ),
        campus(
            "campus_evaluations_submit",
            "提交已经生成的评教预览（写操作）。只有用户看到预览并明确要求提交时才带 confirm=true；结果不明的问卷不会被自动重试。",
            json!({
                "previewId": { "type": "string", "description": "campus_evaluations_preview 返回的 id" },
                "confirm": { "type": "boolean", "description": "默认 false；用户明确确认提交时设为 true" }
            }),
            &["previewId"],
        ),
        campus(
            "campus_notice_channels",
            "列出可读取的校园通知栏目（教务处、学生处、研究生院、网络信息中心）",
            json!({}),
            &[],
        ),
        campus(
            "campus_notices",
            "读取某个校园通知栏目最近发布的公告标题、分类、日期与链接（公开页面，只读）",
            json!({
                "channel": { "type": "string", "description": "栏目编号，来自 campus_notice_channels" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 50, "description": "最多返回条数，默认 20" }
            }),
            &["channel"],
        ),
        campus(
            "campus_profile",
            "查询学籍档案（姓名、学号、学院、专业、班级等）",
            json!({}),
            &[],
        ),
        campus(
            "campus_lms_courses",
            "列出思源学堂课程及各课程的学期、教师信息",
            json!({}),
            &[],
        ),
        campus(
            "campus_lms_activities",
            "列出思源学堂某门课程的全部活动（作业、资料、回放、讨论、问卷等）",
            json!({
                "course": { "type": "integer", "minimum": 1, "description": "课程编号，来自 campus_lms_courses" }
            }),
            &["course"],
        ),
        campus(
            "campus_lms_activity",
            "查看思源学堂活动详情：说明、附件、提交与成绩概况",
            json!({
                "activity": { "type": "integer", "minimum": 1, "description": "活动编号，来自 campus_lms_activities" }
            }),
            &["activity"],
        ),
        campus(
            "campus_lms_submissions",
            "查看思源学堂某次作业的我的提交记录、分数与教师批语（只读）",
            json!({
                "activity": { "type": "integer", "minimum": 1, "description": "作业活动编号，来自 campus_lms_activities" }
            }),
            &["activity"],
        ),
        campus(
            "campus_lms_submit",
            "提交思源学堂作业（写操作）。默认只返回预览，不上传也不提交；预览中核对课程、作业和文件，并向用户复述确认后，用户明确要求提交时才带 confirm=true 再次调用。文件必须是用户明确指定的本地绝对路径，仅支持常见文档、代码和媒体类型，最大 100 MB/个、200 MB/次。不要重试结果不明的提交。",
            json!({
                "activity": { "type": "integer", "minimum": 1, "description": "作业活动编号，来自 campus_lms_activities" },
                "comment": { "type": "string", "description": "文字答案或作业说明，可留空" },
                "files": {
                    "type": "array",
                    "maxItems": 10,
                    "items": { "type": "string" },
                    "description": "要上传的本地文件绝对路径，只用用户明确指定且已核对的文件"
                },
                "draft": { "type": "boolean", "description": "true 保存为草稿；false 正式提交，默认 false" },
                "confirm": { "type": "boolean", "description": "默认 false，只预览；用户明确要求提交且预览无误时设为 true" }
            }),
            &["activity"],
        ),
        campus(
            "campus_library_floors",
            "列出图书馆可选楼层",
            json!({}),
            &[],
        ),
        campus(
            "campus_library_areas",
            "列出图书馆某一楼层的区域",
            json!({
                "floor": { "type": "string", "description": "楼层编号，来自 campus_library_floors" }
            }),
            &["floor"],
        ),
        campus(
            "campus_library_seats",
            "查询图书馆某区域当前可选座位",
            json!({
                "area": { "type": "string", "description": "区域编号，来自 campus_library_areas" }
            }),
            &["area"],
        ),
        campus("campus_venues", "列出可预约的场馆", json!({}), &[]),
        campus(
            "campus_venue_slots",
            "查询某场馆某日期的可预约时段",
            json!({
                "venue": { "type": "integer", "minimum": 1, "description": "场馆编号，来自 campus_venues" },
                "date": date
            }),
            &["venue", "date"],
        ),
        campus(
            "campus_venue_orders",
            "查询我的场馆预约记录",
            json!({}),
            &[],
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

fn id(args: &Value, key: &str) -> Result<u64, String> {
    args.get(key)
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("缺少有效的编号参数 {key}"))
}

fn strings(args: &Value, key: &str) -> Result<Vec<String>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .filter(|item| !item.trim().is_empty())
                    .map(str::to_string)
                    .ok_or_else(|| format!("参数 {key} 必须是字符串列表"))
            })
            .collect(),
        _ => Err(format!("参数 {key} 必须是字符串列表")),
    }
}

fn comments(args: &Value) -> Result<BTreeMap<String, String>, String> {
    match args.get("comments") {
        None | Some(Value::Null) => Ok(BTreeMap::new()),
        Some(Value::Object(items)) => items
            .iter()
            .map(|(key, value)| {
                value
                    .as_str()
                    .map(|value| (key.clone(), value.to_string()))
                    .ok_or_else(|| "参数 comments 必须是问卷编号到评语的映射".to_string())
            })
            .collect(),
        _ => Err("参数 comments 必须是问卷编号到评语的映射".into()),
    }
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
        "campus_evaluations" => school::Query::Evaluations {
            term: text(args, "term"),
            finished: args
                .get("finished")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        },
        "campus_notice_channels" => school::Query::NoticeChannels,
        "campus_notices" => school::Query::Notices {
            channel: required(args, "channel")?,
            limit: args
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(20)
                .clamp(1, 50) as u32,
        },
        "campus_profile" => school::Query::Profile,
        "campus_lms_courses" => school::Query::LmsCourses,
        "campus_lms_activities" => school::Query::LmsActivities {
            course: id(args, "course")?,
        },
        "campus_lms_activity" => school::Query::LmsActivity {
            activity: id(args, "activity")?,
        },
        "campus_lms_submissions" => school::Query::LmsSubmissions {
            activity: id(args, "activity")?,
        },
        "campus_library_floors" => school::Query::LibraryFloors,
        "campus_library_areas" => school::Query::LibraryAreas {
            floor: required(args, "floor")?,
        },
        "campus_library_seats" => school::Query::LibrarySeats {
            area: required(args, "area")?,
        },
        "campus_venues" => school::Query::VenueList,
        "campus_venue_slots" => school::Query::VenueSlots {
            venue: id(args, "venue")?,
            date: required(args, "date")?,
        },
        "campus_venue_orders" => school::Query::VenueOrders,
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
                "campus_evaluations_preview" => {
                    let request = school::evaluation::Prepare {
                        ids: strings(&args, "ids")?,
                        rank: args
                            .get("rank")
                            .and_then(Value::as_u64)
                            .unwrap_or(2)
                            .clamp(1, 5) as u8,
                        comment: text(&args, "comment").unwrap_or_default(),
                        overrides: BTreeMap::new(),
                        comments: comments(&args)?,
                    };
                    let data = crate::prepare_evaluations(app, request).await?;
                    Ok(truncate(
                        serde_json::to_string_pretty(&data).map_err(|_| "无法序列化评教预览")?,
                    ))
                }
                "campus_evaluations_submit" => {
                    let preview_id = required(&args, "previewId")?;
                    let confirm = args
                        .get("confirm")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    if !confirm {
                        return Ok(truncate(
                            serde_json::json!({
                                "confirmRequired": true,
                                "previewId": preview_id,
                                "message": "尚未提交。请把预览中的课程、评分和评语如实告知用户；用户明确要求提交后，再带 confirm=true 调用。",
                            })
                            .to_string(),
                        ));
                    }
                    let data = crate::submit_evaluations(app, preview_id).await?;
                    Ok(truncate(
                        serde_json::to_string_pretty(&data).map_err(|_| "无法序列化评教结果")?,
                    ))
                }
                "campus_lms_submit" => {
                    let request = school::lms::Homework {
                        activity: id(&args, "activity")?,
                        comment: text(&args, "comment").unwrap_or_default(),
                        files: strings(&args, "files")?,
                        draft: args.get("draft").and_then(Value::as_bool).unwrap_or(false),
                    };
                    let confirm = args
                        .get("confirm")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let data = crate::run_lms_homework(app, request, confirm).await?;
                    Ok(truncate(
                        serde_json::to_string_pretty(&data).map_err(|_| "无法序列化提交结果")?,
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
    fn evaluation_comments_are_typed() {
        let parsed = comments(&json!({"comments": {"q": "老师认真负责"}})).unwrap();
        assert_eq!(parsed.get("q").map(String::as_str), Some("老师认真负责"));
        assert!(comments(&json!({})).unwrap().is_empty());
        assert!(comments(&json!({"comments": {"q": 1}})).is_err());
        assert!(comments(&json!({"comments": []})).is_err());
    }

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
        assert!(matches!(
            query_for("campus_lms_activities", &json!({"course": 12})).unwrap(),
            school::Query::LmsActivities { course: 12 }
        ));
        assert!(query_for("campus_lms_activities", &json!({"course": 0})).is_err());
        assert!(matches!(
            query_for("campus_library_seats", &json!({"area": "A"})).unwrap(),
            school::Query::LibrarySeats { .. }
        ));
        assert!(matches!(
            query_for("campus_evaluations", &json!({"finished": true})).unwrap(),
            school::Query::Evaluations { finished: true, .. }
        ));
        let defs = builtin();
        let names: Vec<&str> = defs.iter().map(|def| def.name.as_str()).collect();
        for name in [
            "campus_profile",
            "campus_evaluations_preview",
            "campus_evaluations_submit",
            "campus_notice_channels",
            "campus_notices",
            "campus_lms_courses",
            "campus_lms_activity",
            "campus_lms_submissions",
            "campus_lms_submit",
            "campus_venues",
            "campus_venue_slots",
            "campus_venue_orders",
        ] {
            assert!(names.contains(&name), "{name}");
        }
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
