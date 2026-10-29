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

fn choice(values: &[&str], description: &str) -> Value {
    json!({ "type": "string", "enum": values, "description": description })
}

/// One read-only tool per domain. `action` selects the query; the write tools
/// (`campus_timetable_edit`, `campus_evaluations_submit`, `campus_lms_submit`)
/// stay separate so they can be disabled on the local MCP endpoint without
/// hiding read access.
pub fn builtin() -> Vec<ToolDef> {
    let term =
        json!({ "type": "string", "description": "学期，例如 2025-2026-1；留空使用当前学期" });
    let page = json!({ "type": "integer", "minimum": 1, "maximum": 500, "description": "页码，从 1 开始" });
    let date = json!({ "type": "string", "description": "日期，格式 YYYY-MM-DD" });
    vec![
        campus(
            "campus_timetable",
            "查询课表。action=current（默认）返回当前学期课程；action=customizations 列出本机对课表的修改：改过的教务课程、隐藏的课程和自建课程。",
            json!({
                "action": choice(&["current", "customizations"], "current 查当前课表；customizations 查本机课表修改"),
                "semester": { "type": "string", "description": "customizations 可选，默认当前已同步的学期" }
            }),
            &[],
        ),
        campus(
            "campus_timetable_edit",
            "修改课表（写操作，只影响本机）。save 新增自建课程或修改现有课程；hide 隐藏课程；reset 恢复默认或删除自建课程。先不带 confirm 调用查看预览，用户明确同意后再带 confirm=true。先查 campus_timetable 的 action=current 拿课程 id。",
            json!({
                "action": {
                    "type": "string",
                    "enum": ["save", "hide", "reset"],
                    "description": "save 新增或修改；hide 隐藏；reset 恢复默认或删除自建课程"
                },
                "id": { "type": "string", "description": "campus_timetable 返回的课程 id；action=save 且新增自建课程时省略" },
                "semester": { "type": "string", "description": "可选，默认当前已同步的学期" },
                "course": {
                    "type": "object",
                    "description": "save 时的课程内容",
                    "properties": {
                        "name": { "type": "string", "description": "课程名称" },
                        "teacher": { "type": "string", "description": "教师，可留空" },
                        "location": { "type": "string", "description": "地点，可留空" },
                        "day": { "type": "integer", "minimum": 1, "maximum": 7, "description": "星期，1 为周一" },
                        "start": { "type": "integer", "minimum": 1, "maximum": 20, "description": "开始节次" },
                        "end": { "type": "integer", "minimum": 1, "maximum": 20, "description": "结束节次" },
                        "weeks": { "description": "周次，例如 1-16 或 1-8,10-16；也可传周次数组", "oneOf": [{ "type": "string" }, { "type": "array", "items": { "type": "integer" } }] },
                        "color": { "type": "integer", "minimum": 0, "maximum": 5, "description": "颜色编号 0-5，省略时按名称取色" }
                    },
                    "required": ["name", "day", "start", "end", "weeks"],
                    "additionalProperties": false
                },
                "confirm": { "type": "boolean", "description": "默认 false，只返回预览；用户明确同意后才设为 true" }
            }),
            &["action"],
        ),
        campus(
            "campus_academic",
            "教务查询。action=terms 列出可选学期；grades 查询本科成绩；exams 查询考试安排；calendar 查询校历；profile 查询学籍档案；courses 按名称查询全校课程（需要 name 和 campus）。",
            json!({
                "action": choice(&["terms", "grades", "exams", "calendar", "profile", "courses"], "要查询的内容"),
                "term": term,
                "name": { "type": "string", "description": "courses 必填，课程名称关键词" },
                "campus": { "type": "string", "description": "courses 必填，校区" }
            }),
            &["action"],
        ),
        campus(
            "campus_attendance",
            "考勤查询。action=streams 查询考勤流水，start / end 可选；action=records 按日期范围查询课程考勤，需要 start 和 end。",
            json!({
                "action": choice(&["streams", "records"], "streams 查流水；records 按日期查课程考勤"),
                "start": date,
                "end": date,
                "page": page
            }),
            &["action"],
        ),
        campus(
            "campus_card",
            "校园卡查询。action=balance（默认）查询余额；action=transactions 查询流水（需要 start 和 end）。",
            json!({
                "action": choice(&["balance", "transactions"], "balance 查余额；transactions 查流水"),
                "start": date,
                "end": date,
                "page": page
            }),
            &[],
        ),
        campus(
            "campus_fitness",
            "体测查询。action=overview（默认）查询体测概览；action=score 查询某一学年成绩（需要 year）。",
            json!({
                "action": choice(&["overview", "score"], "overview 查概览；score 查某一学年成绩"),
                "year": { "type": "string", "description": "score 必填，学年，例如 2024-2025" }
            }),
            &[],
        ),
        campus(
            "campus_classroom",
            "教室查询。action=options 列出自习教室查询可用的校区和楼宇；action=rooms 查询空闲教室（需要 campus、building、date、start、end）。",
            json!({
                "action": choice(&["options", "rooms"], "options 查可选校区楼宇；rooms 查空闲教室"),
                "campus": { "type": "string", "description": "rooms 必填，校区" },
                "building": { "type": "string", "description": "rooms 必填，楼宇，来自 action=options" },
                "date": date,
                "start": { "type": "integer", "minimum": 1, "maximum": 14, "description": "rooms 必填，开始节次" },
                "end": { "type": "integer", "minimum": 1, "maximum": 14, "description": "rooms 必填，结束节次" }
            }),
            &["action"],
        ),
        campus(
            "campus_evaluation",
            "评教。action=list（默认）列出教学评价（只读）；action=preview 自动填写待评问卷并生成预览（不会提交）。ids 来自 list 返回的问卷 id；rank 1 最高、5 最低；comments 可按问卷编号为每位老师单独设置文字评语。",
            json!({
                "action": choice(&["list", "preview"], "list 查评教列表；preview 生成预览"),
                "term": term,
                "finished": { "type": "boolean", "description": "list 可选，true 查看已评，false 查看待评" },
                "ids": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 32,
                    "items": { "type": "string" },
                    "description": "preview 必填，问卷编号列表，来自 action=list"
                },
                "rank": { "type": "integer", "minimum": 1, "maximum": 5, "description": "preview 可选，评分档位，1 最高、5 最低，默认 2" },
                "comment": { "type": "string", "description": "preview 可选，默认文字评语，用于所有必填文字题" },
                "comments": {
                    "type": "object",
                    "additionalProperties": { "type": "string" },
                    "description": "preview 可选，按问卷编号覆盖该老师的文字评语"
                }
            }),
            &[],
        ),
        campus(
            "campus_evaluations_submit",
            "提交已经生成的评教预览（写操作）。只有用户看到预览并明确要求提交时才带 confirm=true；结果不明的问卷不会被自动重试。",
            json!({
                "previewId": { "type": "string", "description": "campus_evaluation 的 action=preview 返回的 id" },
                "confirm": { "type": "boolean", "description": "默认 false；用户明确确认提交时设为 true" }
            }),
            &["previewId"],
        ),
        campus(
            "campus_notices",
            "校园通知。action=channels 列出可读取的栏目（教务处、学生处、研究生院、网络信息中心）；action=list（默认）读取某个栏目最近发布的公告（需要 channel）。",
            json!({
                "action": choice(&["channels", "list"], "channels 列栏目；list 读公告"),
                "channel": { "type": "string", "description": "list 必填，栏目编号，来自 action=channels" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 50, "description": "list 可选，最多返回条数，默认 20" }
            }),
            &[],
        ),
        campus(
            "campus_lms",
            "思源学堂（只读）。action=courses 列出课程；activities 列出某门课程的活动（需要 course）；activity 查看活动详情（需要 activity）；submissions 查看某次作业的我的提交记录（需要 activity）。",
            json!({
                "action": choice(&["courses", "activities", "activity", "submissions"], "要查询的内容"),
                "course": { "type": "integer", "minimum": 1, "description": "activities 必填，课程编号，来自 action=courses" },
                "activity": { "type": "integer", "minimum": 1, "description": "activity / submissions 必填，活动编号，来自 action=activities" }
            }),
            &["action"],
        ),
        campus(
            "campus_lms_submit",
            "提交思源学堂作业（写操作）。默认只返回预览，不上传也不提交；预览中核对课程、作业和文件，并向用户复述确认后，用户明确要求提交时才带 confirm=true 再次调用。文件必须是用户明确指定的本地绝对路径，仅支持常见文档、代码和媒体类型，最大 100 MB/个、200 MB/次。不要重试结果不明的提交。",
            json!({
                "activity": { "type": "integer", "minimum": 1, "description": "作业活动编号，来自 campus_lms 的 action=activities" },
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
            "campus_library",
            "图书馆。action=floors（默认）列出可选楼层；areas 列出某楼层区域（需要 floor）；seats 查询某区域当前可选座位（需要 area）。",
            json!({
                "action": choice(&["floors", "areas", "seats"], "要查询的内容"),
                "floor": { "type": "string", "description": "areas 必填，楼层编号，来自 action=floors" },
                "area": { "type": "string", "description": "seats 必填，区域编号，来自 action=areas" }
            }),
            &[],
        ),
        campus(
            "campus_venues",
            "场馆。action=list（默认）列出可预约的场馆；slots 查询某场馆某日期的可预约时段（需要 venue 和 date）；orders 查询我的场馆预约记录。",
            json!({
                "action": choice(&["list", "slots", "orders"], "要查询的内容"),
                "venue": { "type": "integer", "minimum": 1, "description": "slots 必填，场馆编号，来自 action=list" },
                "date": date
            }),
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

fn small(args: &Value, key: &str) -> Result<u8, String> {
    args.get(key)
        .and_then(Value::as_u64)
        .filter(|value| *value <= u64::from(u8::MAX))
        .map(|value| value as u8)
        .ok_or_else(|| format!("缺少有效的整数参数 {key}"))
}

/// The selected action, falling back to the action that keeps a pre-merge
/// call working (`campus_card` without `action` still returns the balance).
fn action_of(args: &Value, default: &str) -> String {
    text(args, "action").unwrap_or_else(|| default.to_string())
}

fn invalid_action(tool: &str, action: &str, allowed: &str) -> String {
    format!("{tool} 不支持 action={action}，可选 {allowed}")
}

fn timetable_request(args: &Value) -> Result<crate::timetable_custom::Request, String> {
    use crate::timetable_custom::{Action, Draft, Request, Weeks};
    let action = match required(args, "action")?.as_str() {
        "save" => Action::Save,
        "hide" => Action::Hide,
        "reset" => Action::Reset,
        other => return Err(format!("不支持的操作 {other}，只能是 save、hide 或 reset")),
    };
    let course = match args.get("course") {
        None | Some(Value::Null) => None,
        Some(value) if value.is_object() => {
            let weeks = match value.get("weeks") {
                Some(Value::String(text)) => Weeks::Text(text.clone()),
                Some(Value::Array(items)) => Weeks::List(
                    items
                        .iter()
                        .map(|item| {
                            item.as_u64()
                                .and_then(|value| u8::try_from(value).ok())
                                .ok_or_else(|| {
                                    "参数 course.weeks 必须是 1 到 60 的整数列表".to_string()
                                })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                ),
                _ => return Err("参数 course.weeks 必须是文本或整数列表".into()),
            };
            Some(Draft {
                name: required(value, "name")?,
                teacher: text(value, "teacher").unwrap_or_default(),
                location: text(value, "location").unwrap_or_default(),
                day: small(value, "day")?,
                start: small(value, "start")?,
                end: small(value, "end")?,
                weeks,
                color: match value.get("color") {
                    None | Some(Value::Null) => None,
                    Some(_) => Some(small(value, "color")?),
                },
            })
        }
        Some(_) => return Err("参数 course 必须是对象".into()),
    };
    if action == Action::Save && course.is_none() {
        return Err("save 需要 course 内容".into());
    }
    Ok(Request {
        semester: text(args, "semester").unwrap_or_default(),
        action,
        id: text(args, "id").unwrap_or_default(),
        course,
    })
}

fn query_for(name: &str, args: &Value) -> Result<school::Query, String> {
    let query = match name {
        "campus_academic" => match action_of(args, "").as_str() {
            "terms" => school::Query::Terms,
            "grades" => school::Query::Grades {
                term: text(args, "term"),
            },
            "exams" => school::Query::Exams {
                term: text(args, "term"),
            },
            "calendar" => school::Query::Calendar,
            "profile" => school::Query::Profile,
            "courses" => school::Query::Courses {
                term: text(args, "term"),
                name: required(args, "name")?,
                campus: required(args, "campus")?,
            },
            other => {
                return Err(invalid_action(
                    "campus_academic",
                    other,
                    "terms、grades、exams、calendar、profile、courses",
                ))
            }
        },
        "campus_attendance" => match action_of(args, "").as_str() {
            "streams" => school::Query::AttendanceStreams {
                start: text(args, "start"),
                end: text(args, "end"),
                page: number(args, "page", 1),
            },
            "records" => school::Query::AttendanceRecords {
                start: required(args, "start")?,
                end: required(args, "end")?,
                page: number(args, "page", 1),
            },
            other => {
                return Err(invalid_action(
                    "campus_attendance",
                    other,
                    "streams、records",
                ))
            }
        },
        "campus_card" => match action_of(args, "balance").as_str() {
            "balance" => school::Query::Card,
            "transactions" => school::Query::CardTransactions {
                start: required(args, "start")?,
                end: required(args, "end")?,
                page: number(args, "page", 1),
            },
            other => {
                return Err(invalid_action(
                    "campus_card",
                    other,
                    "balance、transactions",
                ))
            }
        },
        "campus_fitness" => match action_of(args, "overview").as_str() {
            "overview" => school::Query::Fitness,
            "score" => school::Query::FitnessScore {
                year: required(args, "year")?,
            },
            other => return Err(invalid_action("campus_fitness", other, "overview、score")),
        },
        "campus_classroom" => match action_of(args, "").as_str() {
            "options" => school::Query::RoomOptions,
            "rooms" => school::Query::Rooms {
                campus: required(args, "campus")?,
                building: required(args, "building")?,
                date: required(args, "date")?,
                start: number(args, "start", 0) as u8,
                end: number(args, "end", 0) as u8,
            },
            other => return Err(invalid_action("campus_classroom", other, "options、rooms")),
        },
        "campus_evaluation" => match action_of(args, "list").as_str() {
            "list" => school::Query::Evaluations {
                term: text(args, "term"),
                finished: args
                    .get("finished")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            },
            other => {
                return Err(invalid_action(
                    "campus_evaluation",
                    other,
                    "list；preview 由预览流程处理",
                ))
            }
        },
        "campus_notices" => match action_of(args, "list").as_str() {
            "channels" => school::Query::NoticeChannels,
            "list" => school::Query::Notices {
                channel: required(args, "channel")?,
                limit: args
                    .get("limit")
                    .and_then(Value::as_u64)
                    .unwrap_or(20)
                    .clamp(1, 50) as u32,
            },
            other => return Err(invalid_action("campus_notices", other, "channels、list")),
        },
        "campus_lms" => match action_of(args, "").as_str() {
            "courses" => school::Query::LmsCourses,
            "activities" => school::Query::LmsActivities {
                course: id(args, "course")?,
            },
            "activity" => school::Query::LmsActivity {
                activity: id(args, "activity")?,
            },
            "submissions" => school::Query::LmsSubmissions {
                activity: id(args, "activity")?,
            },
            other => {
                return Err(invalid_action(
                    "campus_lms",
                    other,
                    "courses、activities、activity、submissions",
                ))
            }
        },
        "campus_library" => match action_of(args, "floors").as_str() {
            "floors" => school::Query::LibraryFloors,
            "areas" => school::Query::LibraryAreas {
                floor: required(args, "floor")?,
            },
            "seats" => school::Query::LibrarySeats {
                area: required(args, "area")?,
            },
            other => {
                return Err(invalid_action(
                    "campus_library",
                    other,
                    "floors、areas、seats",
                ))
            }
        },
        "campus_venues" => match action_of(args, "list").as_str() {
            "list" => school::Query::VenueList,
            "slots" => school::Query::VenueSlots {
                venue: id(args, "venue")?,
                date: required(args, "date")?,
            },
            "orders" => school::Query::VenueOrders,
            other => {
                return Err(invalid_action(
                    "campus_venues",
                    other,
                    "list、slots、orders",
                ))
            }
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
                "campus_timetable" => match action_of(&args, "current").as_str() {
                    "current" => {
                        let timetable = crate::run_timetable(app).await?;
                        Ok(truncate(
                            serde_json::to_string(&timetable).map_err(|_| "无法序列化课表")?,
                        ))
                    }
                    "customizations" => {
                        let data = crate::timetable_customizations(app, text(&args, "semester"))?;
                        Ok(truncate(
                            serde_json::to_string_pretty(&data)
                                .map_err(|_| "无法序列化课表修改记录")?,
                        ))
                    }
                    other => Err(invalid_action(
                        "campus_timetable",
                        other,
                        "current、customizations",
                    )),
                },
                "campus_timetable_edit" => {
                    let confirm = args
                        .get("confirm")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let data =
                        crate::apply_timetable_edit(app, timetable_request(&args)?, confirm)?;
                    Ok(truncate(
                        serde_json::to_string_pretty(&data)
                            .map_err(|_| "无法序列化课表修改结果")?,
                    ))
                }
                "campus_evaluation" => match action_of(&args, "list").as_str() {
                    "preview" => {
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
                            serde_json::to_string_pretty(&data)
                                .map_err(|_| "无法序列化评教预览")?,
                        ))
                    }
                    "list" => {
                        let query = query_for(name, &args)?;
                        let data = crate::run_school_query(app, query, None).await?;
                        Ok(truncate(
                            serde_json::to_string(&data).map_err(|_| "无法序列化查询结果")?,
                        ))
                    }
                    other => Err(invalid_action("campus_evaluation", other, "list、preview")),
                },
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

/// Translate a saved MCP call before dispatch. The stored arguments are kept
/// and the merged action is added, so old clients do not need to change.
pub fn legacy(name: &str, args: &Value) -> Option<(&'static str, Value)> {
    let (tool, action) = crate::tool_names::merged(name)?;
    let mut args = args.clone();
    if let Some(action) = action {
        if let Some(map) = args.as_object_mut() {
            map.insert("action".into(), json!(action));
        }
    }
    Some((tool, args))
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
    fn timetable_edits_are_typed() {
        let request = timetable_request(&json!({
            "action": "save",
            "course": {
                "name": "自习",
                "day": 3,
                "start": 3,
                "end": 4,
                "weeks": "1-4,6"
            }
        }))
        .unwrap();
        let course = request.course.unwrap();
        assert_eq!(course.name, "自习");
        assert!(
            matches!(&course.weeks, crate::timetable_custom::Weeks::Text(text) if text == "1-4,6")
        );
        assert!(request.id.is_empty());

        let listed = timetable_request(&json!({
            "action": "save",
            "course": {"name": "自习", "day": 3, "start": 3, "end": 4, "weeks": [1, 2]}
        }))
        .unwrap();
        assert!(
            matches!(&listed.course.unwrap().weeks, crate::timetable_custom::Weeks::List(weeks) if weeks == &vec![1, 2])
        );

        assert!(timetable_request(&json!({"action": "delete"})).is_err());
        assert!(timetable_request(&json!({"action": "save"})).is_err());
        assert!(timetable_request(&json!({"action": "hide"})).is_ok());
        assert!(timetable_request(
            &json!({"action": "save", "course": {"name": "x", "day": 1, "start": 1, "end": 2}})
        )
        .is_err());
    }

    #[test]
    fn maps_actions_to_queries() {
        assert!(matches!(
            query_for(
                "campus_academic",
                &json!({"action": "grades", "term": "2025-2026-1"})
            )
            .unwrap(),
            school::Query::Grades { .. }
        ));
        assert!(matches!(
            query_for("campus_academic", &json!({"action": "profile"})).unwrap(),
            school::Query::Profile
        ));
        assert!(matches!(
            query_for("campus_card", &json!({})).unwrap(),
            school::Query::Card
        ));
        assert!(matches!(
            query_for(
                "campus_card",
                &json!({"action": "transactions", "start": "2026-01-01", "end": "2026-01-31"})
            )
            .unwrap(),
            school::Query::CardTransactions { .. }
        ));
        assert!(query_for("campus_attendance", &json!({"action": "records"})).is_err());
        assert!(query_for("campus_attendance", &json!({"action": "streams"})).is_ok());
        assert!(query_for("nope", &json!({})).is_err());
        assert!(matches!(
            query_for("campus_lms", &json!({"action": "activities", "course": 12})).unwrap(),
            school::Query::LmsActivities { course: 12 }
        ));
        assert!(query_for("campus_lms", &json!({"action": "activities", "course": 0})).is_err());
        assert!(matches!(
            query_for("campus_library", &json!({"action": "seats", "area": "A"})).unwrap(),
            school::Query::LibrarySeats { .. }
        ));
        assert!(matches!(
            query_for("campus_evaluation", &json!({"finished": true})).unwrap(),
            school::Query::Evaluations { finished: true, .. }
        ));
        assert!(matches!(
            query_for("campus_venues", &json!({})).unwrap(),
            school::Query::VenueList
        ));
        assert!(matches!(
            query_for("campus_notices", &json!({"channel": "dean"})).unwrap(),
            school::Query::Notices { .. }
        ));
    }

    #[test]
    fn merged_tools_cover_every_query_and_stay_small() {
        let defs = builtin();
        let names: Vec<&str> = defs.iter().map(|def| def.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "campus_timetable",
                "campus_timetable_edit",
                "campus_academic",
                "campus_attendance",
                "campus_card",
                "campus_fitness",
                "campus_classroom",
                "campus_evaluation",
                "campus_evaluations_submit",
                "campus_notices",
                "campus_lms",
                "campus_lms_submit",
                "campus_library",
                "campus_venues",
            ]
        );
        for def in &defs {
            assert_eq!(def.parameters["type"], "object");
        }
    }

    #[test]
    fn legacy_names_are_translated_to_merged_actions() {
        let (tool, args) = legacy("campus_grades", &json!({"term": "2025-2026-1"})).unwrap();
        assert_eq!(tool, "campus_academic");
        assert_eq!(args["action"], "grades");
        assert_eq!(args["term"], "2025-2026-1");

        let (tool, args) = legacy("campus_evaluations_preview", &json!({"ids": ["q"]})).unwrap();
        assert_eq!(tool, "campus_evaluation");
        assert_eq!(args["action"], "preview");

        assert!(legacy("campus_card", &json!({})).is_none());
        assert!(legacy("campus_grades", &json!({})).is_some());
        assert_eq!(
            crate::tool_names::canonical_name("campus_card_transactions"),
            "campus_card"
        );
        assert_eq!(
            crate::tool_names::canonical_name("campus_card"),
            "campus_card"
        );
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
