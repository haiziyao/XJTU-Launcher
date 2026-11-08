//! 思源学堂 adapter. Reads use the saved `sites.lms` cookie session. Homework
//! submission is the only write path: it uploads user-chosen local files and
//! posts one submission, never retrying when the outcome is unknown.
use crate::{
    accounts::Account,
    network::{self, AccessMode, Transport},
};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::{Component, Path, PathBuf},
    time::Duration,
};
use url::Url;

const BASE: &str = "https://lms.xjtu.edu.cn";
const MEDIA_HOST: &str = "media.xjtu.edu.cn";
const PLAYER_HOST: &str = "rms-v5.xjtu.edu.cn";
/// Replay videos are served from presigned S3 URLs on this host.
const REVIEW_HOST: &str = "review-class.xjtu.edu.cn";
/// Attachment and replay downloads are user-initiated; keep a hard ceiling so a
/// wrong link cannot fill the disk.
const MAX_DOWNLOAD: u64 = 2 * 1024 * 1024 * 1024;
/// Homework attachments are user-chosen; bound each upload and the whole call.
const MAX_UPLOAD: u64 = 100 * 1024 * 1024;
const MAX_UPLOAD_TOTAL: u64 = 200 * 1024 * 1024;
pub(crate) const MAX_UPLOAD_FILES: usize = 10;
const MAX_COMMENT: usize = 10_000;
pub(crate) const LOGIN_URL: &str = BASE;

/// Only document, media and code files. This keeps a prompt-injected tool call
/// from turning homework upload into a way to exfiltrate credentials such as
/// `auth.json` or extension-less private keys.
const UPLOAD_EXTENSIONS: &[&str] = &[
    "pdf", "doc", "docx", "ppt", "pptx", "xls", "xlsx", "txt", "md", "csv", "rtf", "odt", "ods",
    "odp", "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "png", "jpg", "jpeg", "gif", "bmp",
    "webp", "svg", "mp4", "mov", "avi", "mkv", "mp3", "wav", "m4a", "flac", "c", "cc", "cpp", "h",
    "hpp", "py", "java", "js", "ts", "m", "mm", "rs", "go", "sql", "tex", "bib", "ipynb", "html",
    "css", "xml", "yaml", "yml",
];

/// A homework submission as the Agent asked for it. Files stay as paths until
/// the preview or submit step resolves and checks them.
#[derive(Clone, Debug)]
pub struct Homework {
    pub activity: u64,
    pub comment: String,
    pub files: Vec<String>,
    pub draft: bool,
}

struct UploadFile {
    path: PathBuf,
    name: String,
    size: u64,
}

/// One replay video as the capture page needs it. `download_url` is a
/// short-lived presigned address: it must stay out of logs, caches, error
/// messages and screenshots.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayVideo {
    pub label: String,
    pub size: u64,
    pub mute: bool,
    pub best_audio: bool,
    pub download_url: String,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayVideos {
    pub activity: u64,
    pub title: String,
    pub videos: Vec<ReplayVideo>,
}

impl Homework {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=9_999_999_999).contains(&self.activity) {
            return Err("作业编号无效".into());
        }
        if self.files.len() > MAX_UPLOAD_FILES {
            return Err(format!("一次最多上传 {MAX_UPLOAD_FILES} 个文件"));
        }
        if self.comment.chars().count() > MAX_COMMENT {
            return Err("作业文字内容过长".into());
        }
        if self.comment.contains('\0') {
            return Err("作业文字内容无效".into());
        }
        if self.comment.trim().is_empty() && self.files.is_empty() {
            return Err("请至少提供一个文件或一段文字".into());
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Client {
    transport: Transport,
    mode: AccessMode,
}

/// True when the school will not accept a submission right now: the activity is
/// closed and no resubmission window is open. Drafts are always allowed.
fn submission_blocked(activity: &Value, draft: bool) -> bool {
    if draft {
        return false;
    }
    let closed = activity
        .get("is_closed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let resubmit_open = activity
        .get("is_resubmit_open")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    closed && !resubmit_open
}

fn text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

/// 思源学堂 returns UTC ISO times ("…Z"). Convert zoned values to campus time
/// (UTC+8, no daylight saving) so pages, the Agent and MCP read the same
/// clock; values without a zone are already local and pass through.
fn campus_time(value: Option<&Value>) -> String {
    let raw = text(value);
    local_time(&raw).unwrap_or(raw)
}

fn local_time(raw: &str) -> Option<String> {
    let (date, clock) = raw.split_once('T')?;
    let mut ymd = date.split('-').map(|part| part.parse::<i64>().ok());
    let (year, month, day) = (ymd.next()??, ymd.next()??, ymd.next()??);
    let (clock, offset) = if let Some(clock) = clock.strip_suffix(['Z', 'z']) {
        (clock, 0)
    } else {
        let at = clock.rfind(['+', '-'])?;
        let (clock, zone) = clock.split_at(at);
        let digits: String = zone[1..].chars().filter(|c| *c != ':').collect();
        if digits.len() != 4 {
            return None;
        }
        let minutes = digits[..2].parse::<i64>().ok()? * 60 + digits[2..].parse::<i64>().ok()?;
        (
            clock,
            if zone.starts_with('-') {
                -minutes
            } else {
                minutes
            },
        )
    };
    let mut hms = clock.split(':');
    let hour = hms.next()?.parse::<i64>().ok()?;
    let minute = hms.next()?.parse::<i64>().ok()?;
    let second = hms
        .next()
        .map(|part| part.get(..2).unwrap_or(part).parse::<i64>().ok())
        .unwrap_or(Some(0))?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || !(0..24).contains(&hour)
        || !(0..60).contains(&minute)
        || !(0..=60).contains(&second)
    {
        return None;
    }
    // Howard Hinnant's days-from-civil and civil-from-days.
    let (y, m) = if month <= 2 {
        (year - 1, month + 9)
    } else {
        (year, month - 3)
    };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + day - 1;
    let days = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    let total = days * 1440 + hour * 60 + minute - offset + 8 * 60;
    let (z, minutes) = (total.div_euclid(1440) + 719_468, total.rem_euclid(1440));
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    Some(format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{second:02}",
        minutes / 60,
        minutes % 60
    ))
}

/// Today in campus time (UTC+8), used to pick the courses that are running now.
fn campus_today() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0);
    let days = (seconds + 8 * 3600).div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

fn course_day(value: Option<&Value>) -> String {
    let raw = text(value);
    let day = raw.get(..10).unwrap_or(&raw);
    let bytes = day.as_bytes();
    if bytes.len() == 10 && bytes[4] == b'-' && bytes[7] == b'-' {
        day.to_string()
    } else {
        String::new()
    }
}

/// Days since 1970-01-01 for a `YYYY-MM-DD` string; lets the current-term rule
/// group courses whose start dates are a few days apart.
fn day_number(day: &str) -> Option<i64> {
    let mut parts = day.split('-').map(|part| part.parse::<i64>().ok());
    let (year, month, date) = (parts.next()??, parts.next()??, parts.next()??);
    if !(1..=12).contains(&month) || !(1..=31).contains(&date) {
        return None;
    }
    let (y, m) = if month <= 2 {
        (year - 1, month + 9)
    } else {
        (year, month - 3)
    };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + date - 1;
    Some(era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468)
}

/// Courses scheduled for today. Most LMS courses have an empty end date, so
/// when no course is explicitly running the newest start-date cohort (within a
/// month) is used. That keeps the aggregate on one term instead of scanning
/// every semester ever taken.
fn current_course_group(courses: &[Value], today: &str) -> Vec<Value> {
    let today = day_number(today).unwrap_or(i64::MAX);
    // Courses that explicitly cover today define the term. Other courses in
    // the same term often have an empty end date, so include the whole start
    // window around the running ones instead of only the dated courses.
    let active_starts: Vec<i64> = courses
        .iter()
        .filter_map(|course| {
            let start = day_number(&course_day(course.get("start")))?;
            let end = day_number(&course_day(course.get("end")))?;
            (start <= today && today <= end).then_some(start)
        })
        .collect();
    if let (Some(low), Some(high)) = (active_starts.iter().min(), active_starts.iter().max()) {
        return courses
            .iter()
            .filter(|course| {
                day_number(&course_day(course.get("start")))
                    .is_some_and(|start| ((*low - 30)..=(*high + 30)).contains(&start))
            })
            .cloned()
            .collect();
    }
    let anchor = courses
        .iter()
        .filter_map(|course| {
            let start = day_number(&course_day(course.get("start")))?;
            (start <= today).then_some(start)
        })
        .max()
        .or_else(|| {
            courses
                .iter()
                .filter_map(|course| day_number(&course_day(course.get("start"))))
                .filter(|start| *start > today)
                .min()
        });
    let Some(anchor) = anchor else {
        return courses.to_vec();
    };
    let group: Vec<Value> = courses
        .iter()
        .filter(|course| {
            day_number(&course_day(course.get("start")))
                .is_some_and(|start| (start - anchor).abs() <= 30)
        })
        .cloned()
        .collect();
    if group.is_empty() {
        courses.to_vec()
    } else {
        group
    }
}

fn number(value: Option<&Value>) -> Option<f64> {
    let value = value?;
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
        .filter(|value| value.is_finite())
}

fn url(path: &str) -> Result<Url, String> {
    if !path.starts_with("/api/") || path.contains("..") {
        return Err("思源学堂接口路径无效".into());
    }
    Url::parse(&format!("{BASE}{path}")).map_err(|_| "思源学堂接口地址无效".into())
}

/// The download blob redirects to the media host; only that exact HTTPS origin
/// is accepted, so a tampered response cannot point the download elsewhere.
/// Through WebVPN the redirect may be either the plain media URL or an already
/// mapped one, so the origin is checked after decoding and the request is sent
/// through the same mapping.
fn media_url(location: &str, mode: AccessMode) -> Result<Url, String> {
    let url = Url::parse(location).map_err(|_| "附件地址无效")?;
    let original = if mode == AccessMode::Webvpn {
        network::original_url(&url).unwrap_or_else(|_| url.clone())
    } else {
        url
    };
    if original.scheme() != "https"
        || original.host_str() != Some(MEDIA_HOST)
        || original.port_or_known_default() != Some(443)
        || !original.username().is_empty()
        || original.password().is_some()
    {
        return Err("附件地址不受支持".into());
    }
    if mode == AccessMode::Webvpn {
        network::vpn_url(&original)
    } else {
        Ok(original)
    }
}

fn player_url(value: &str) -> Option<String> {
    let url = Url::parse(value).ok()?;
    if url.scheme() == "https"
        && url.port_or_known_default() == Some(443)
        && url.host_str().is_some_and(|host| {
            host == PLAYER_HOST || host == "lms.xjtu.edu.cn" || host.ends_with(".xjtu.edu.cn")
        })
        && url.username().is_empty()
        && url.password().is_none()
    {
        Some(url.to_string())
    } else {
        None
    }
}

/// Presigned replay URLs must stay on the review host. Anything else is
/// dropped instead of being handed to the webview.
fn replay_video(value: &Value) -> Option<ReplayVideo> {
    let address = value
        .get("download_url")
        .or_else(|| value.get("downloadUrl"))
        .and_then(Value::as_str)?;
    let url = Url::parse(address).ok()?;
    if url.scheme() != "https"
        || url.host_str() != Some(REVIEW_HOST)
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    Some(ReplayVideo {
        label: value
            .get("label")
            .or_else(|| value.get("name"))
            .and_then(Value::as_str)
            .filter(|label| !label.trim().is_empty())
            .unwrap_or("回放")
            .to_string(),
        size: value.get("size").and_then(Value::as_u64).unwrap_or(0),
        mute: value.get("mute").and_then(Value::as_bool).unwrap_or(false),
        best_audio: value
            .get("is_best_audio")
            .or_else(|| value.get("isBestAudio"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        download_url: url.to_string(),
    })
}

impl Client {
    pub fn connect(account: &Account) -> Result<Self, String> {
        let site = account.sites.get("lms").ok_or("请先完成思源学堂登录")?;
        let cookies = site
            .cookies
            .as_deref()
            .filter(|cookies| !cookies.is_empty())
            .ok_or("请先完成思源学堂登录")?;
        Self::with_cookies(cookies, account.mode)
    }

    fn with_cookies(cookies: &str, mode: AccessMode) -> Result<Self, String> {
        Ok(Self {
            transport: Transport::new(Some(cookies))?,
            mode,
        })
    }

    async fn send(&self, request: reqwest::RequestBuilder) -> Result<Value, String> {
        let response = request
            .header("Referer", format!("{BASE}/"))
            .header("Accept", "application/json")
            .header("X-Requested-With", "XMLHttpRequest")
            .send()
            .await
            .map_err(|_| "无法连接思源学堂，请检查网络后重试")?;
        let status = response.status();
        if status.is_redirection() || matches!(status.as_u16(), 401 | 403) {
            return Err("思源学堂登录已失效，请刷新后重试".into());
        }
        if !status.is_success() {
            return Err(format!("思源学堂服务暂不可用（HTTP {}）", status.as_u16()));
        }
        response
            .json()
            .await
            .map_err(|_| "思源学堂返回了非预期数据".into())
    }

    async fn get(&self, path: &str) -> Result<Value, String> {
        let address = Transport::address(&url(path)?, self.mode)?;
        self.send(self.transport.client.get(address)).await
    }

    async fn post(&self, path: &str) -> Result<Value, String> {
        let address = Transport::address(&url(path)?, self.mode)?;
        self.send(self.transport.client.post(address)).await
    }

    async fn post_json(&self, path: &str, body: &Value) -> Result<Value, String> {
        let address = Transport::address(&url(path)?, self.mode)?;
        self.send(self.transport.client.post(address).json(body))
            .await
    }

    pub async fn courses(&self) -> Result<Value, String> {
        let value = self.post("/api/my-courses").await?;
        let courses = value
            .get("courses")
            .and_then(Value::as_array)
            .ok_or("思源学堂课程列表格式已变化")?;
        Ok(json!({
            "courses": courses.iter().filter_map(course).collect::<Vec<_>>()
        }))
    }

    pub async fn activities(&self, course: u64) -> Result<Value, String> {
        let value = self
            .get(&format!("/api/courses/{course}/activities"))
            .await?;
        let activities = value
            .get("activities")
            .and_then(Value::as_array)
            .ok_or("思源学堂活动列表格式已变化")?;
        Ok(json!({
            "course": course,
            "activities": activities.iter().filter_map(activity).collect::<Vec<_>>()
        }))
    }

    pub async fn activity(&self, id: u64) -> Result<Value, String> {
        let value = self.get(&format!("/api/activities/{id}")).await?;
        let mut detail = activity_detail(&value);
        if detail["type"] == "lesson" && !detail["replayCode"].as_str().unwrap_or("").is_empty() {
            if let Ok(url) = self.player(id).await {
                detail["player"] = json!(url);
            }
        }
        Ok(detail)
    }

    /// Resolve the direct replay videos for one lesson. The LMS player URL
    /// carries a short-lived token; RMS exchanges it for a bearer token and
    /// returns presigned S3 `download_url`s that need no cookies. The bearer
    /// token stays here, and only the video addresses cross the IPC boundary.
    pub async fn replay_videos(&self, activity: u64) -> Result<ReplayVideos, String> {
        if !(1..=9_999_999_999).contains(&activity) {
            return Err("回放编号无效".into());
        }
        let detail = self.activity(activity).await?;
        if detail["type"] != "lesson" {
            return Err("该活动不是课程回放".into());
        }
        let replay = detail["replayCode"].as_str().unwrap_or("");
        let player = detail["player"].as_str().unwrap_or("");
        if replay.is_empty() || player.is_empty() {
            return Err("这节课暂时没有可用的回放".into());
        }
        let token = Url::parse(player)
            .ok()
            .and_then(|url| {
                url.query_pairs()
                    .find(|(key, _)| key == "token")
                    .map(|(_, value)| value.into_owned())
            })
            .filter(|value| !value.is_empty())
            .ok_or("回放地址缺少令牌")?;
        let response = self
            .transport
            .client
            .get("https://rms-v5.xjtu.edu.cn/api/v1/auth/embed-token")
            .query(&[("token", token)])
            .send()
            .await
            .map_err(|_| "RMS 令牌服务连接失败")?;
        let value: Value = response.json().await.map_err(|_| "RMS 令牌响应格式异常")?;
        let rms = value
            .pointer("/data")
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty())
            .ok_or("RMS 未返回令牌")?;
        let response = self
            .transport
            .client
            .get(format!(
                "https://rms-v5.xjtu.edu.cn/api/embed/lesson-activities/captures/{replay}"
            ))
            .query(&[("lesson_activity_id", activity.to_string())])
            .header("Authorization", format!("Bearer {rms}"))
            .send()
            .await
            .map_err(|_| "回放视频服务连接失败")?;
        let status = response.status();
        let value: Value = response.json().await.map_err(|_| "回放视频响应格式异常")?;
        if !status.is_success() {
            return Err(format!("回放视频服务返回 HTTP {status}"));
        }
        let videos = value
            .pointer("/data/lesson_videos")
            .or_else(|| value.get("lesson_videos"))
            .and_then(Value::as_array)
            .ok_or("回放视频响应格式异常")?
            .iter()
            .filter_map(replay_video)
            .collect::<Vec<_>>();
        if videos.is_empty() {
            return Err("这节课没有可播放的回放视频".into());
        }
        Ok(ReplayVideos {
            activity,
            title: detail["title"].as_str().unwrap_or("").to_string(),
            videos,
        })
    }

    /// Read the current user's submissions for one homework, including the
    /// teacher's score and comments. This is read-only.
    pub async fn submissions(&self, activity: u64) -> Result<Value, String> {
        let detail = self.get(&format!("/api/activities/{activity}")).await?;
        if text(detail.get("type")) != "homework" {
            return Err("该活动不是作业，没有提交记录".into());
        }
        let group = detail
            .get("submit_by_group")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let path = if group {
            let group_id = detail
                .get("group_id")
                .and_then(Value::as_u64)
                .or_else(|| detail.pointer("/data/group_id").and_then(Value::as_u64))
                .filter(|id| *id > 0)
                .ok_or("这是一次小组作业，但思源学堂没有返回小组编号")?;
            format!("/api/activities/{activity}/groups/{group_id}/submission_list")
        } else {
            let profile = self.get("/api/profile").await?;
            let user = profile
                .get("id")
                .and_then(Value::as_u64)
                .filter(|id| *id > 0)
                .ok_or("无法获取思源学堂用户编号")?;
            format!("/api/activities/{activity}/students/{user}/submission_list")
        };
        let value = self.get(&path).await?;
        let list = value
            .get("list")
            .and_then(Value::as_array)
            .ok_or("思源学堂提交记录格式已变化")?;
        Ok(json!({
            "activity": activity,
            "title": text(detail.get("title")),
            "courseId": detail.get("course_id").and_then(Value::as_u64),
            "end": campus_time(detail.get("end_time")),
            "closed": detail.get("is_closed").and_then(Value::as_bool).unwrap_or(false),
            "submitByGroup": group,
            "submissions": list.iter().filter_map(submission).collect::<Vec<_>>(),
        }))
    }

    /// One scan of the current semester's activity lists and details, feeding
    /// both 待办 and 资料库. Doing it in one request means opening the two pages
    /// shares one cache entry instead of asking the LMS twice.
    pub async fn overview(&self) -> Result<Value, String> {
        let courses = self.current_courses().await?;
        // Courses in one cohort can carry different academic-year labels; show
        // the label most of them agree on. `termLabel` is the derived semester
        // (`2026-2027 秋季学期`), because 思源学堂 usually leaves `term` empty.
        let mut labels: HashMap<String, usize> = HashMap::new();
        for course in &courses {
            let label = course_semester_label(course);
            if !label.is_empty() {
                *labels.entry(label).or_insert(0) += 1;
            }
        }
        let semester = labels
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(label, _)| label)
            .unwrap_or_default();
        let (lists, mut failed) = self.activity_lists(&courses).await;
        let mut owners = HashMap::new();
        let mut ids = Vec::new();
        for (course, items) in lists {
            let course_id = course["id"].as_u64().unwrap_or(0);
            let course_name = text(course.get("name"));
            let course_semester = course_semester_label(&course);
            for item in items {
                let kind = text(item.get("type"));
                if !matches!(kind.as_str(), "homework" | "material")
                    || item.get("published").and_then(Value::as_bool) == Some(false)
                {
                    continue;
                }
                let Some(id) = item.get("id").and_then(Value::as_u64) else {
                    continue;
                };
                ids.push(id);
                owners.insert(
                    id,
                    (course_id, course_name.clone(), course_semester.clone()),
                );
            }
        }
        let (details, detail_failures) = self.details_of(&ids).await;
        failed += detail_failures;
        let mut seen = HashSet::new();
        let mut rows = Vec::new();
        let mut files = Vec::new();
        for detail in details {
            let activity = detail["id"].as_u64().unwrap_or(0);
            let (course_id, course_name, course_semester) = owners
                .get(&activity)
                .cloned()
                .unwrap_or((0, String::new(), String::new()));
            match text(detail.get("type")).as_str() {
                "homework" => {
                    let submitted = detail["submitCount"].as_u64().unwrap_or(0) > 0;
                    if submitted || detail["closed"].as_bool().unwrap_or(false) {
                        continue;
                    }
                    rows.push(json!({
                        "id": activity,
                        "courseId": course_id,
                        "course": course_name,
                        "semester": course_semester,
                        "title": detail["title"],
                        "start": detail["start"],
                        "end": detail["end"],
                        "submitByGroup": detail["submitByGroup"],
                        "submitTimes": detail["submitTimes"],
                        "submitCount": detail["submitCount"],
                    }));
                }
                "material" => {
                    for upload in detail["uploads"].as_array().into_iter().flatten() {
                        let Some(id) = upload.get("id").and_then(Value::as_u64) else {
                            continue;
                        };
                        if !seen.insert(id) {
                            continue;
                        }
                        files.push(json!({
                            "courseId": course_id,
                            "course": course_name,
                            "semester": course_semester,
                            "activity": activity,
                            "title": detail["title"],
                            "end": detail["end"],
                            "file": upload,
                        }));
                    }
                }
                _ => {}
            }
        }
        rows.sort_by(|left, right| {
            let left_end = left["end"].as_str().unwrap_or("");
            let right_end = right["end"].as_str().unwrap_or("");
            match (left_end.is_empty(), right_end.is_empty()) {
                (true, true) => std::cmp::Ordering::Equal,
                (true, false) => std::cmp::Ordering::Greater,
                (false, true) => std::cmp::Ordering::Less,
                _ => left_end.cmp(right_end),
            }
        });
        files.sort_by(|left, right| {
            let course = text(left.get("course")).cmp(&text(right.get("course")));
            course
                .then_with(|| text(left.get("title")).cmp(&text(right.get("title"))))
                .then_with(|| {
                    text(left.pointer("/file/name")).cmp(&text(right.pointer("/file/name")))
                })
        });
        Ok(json!({
            "semester": semester,
            "rows": rows,
            "files": files,
            "failed": failed,
        }))
    }

    /// The current term's courses with every homework activity, used by the
    /// 工作台 sync to create course folders and keep an assignment list.
    pub async fn workbench(&self) -> Result<Value, String> {
        let all = self.courses().await?;
        let all = all["courses"].as_array().cloned().unwrap_or_default();
        let group = current_course_group(&all, &campus_today());
        let mut labels: HashMap<String, usize> = HashMap::new();
        for course in &group {
            let label = text(course.get("termLabel"));
            if !label.is_empty() {
                *labels.entry(label).or_insert(0) += 1;
            }
        }
        let term = labels
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(label, _)| label)
            .unwrap_or_default();
        let courses: Vec<Value> = group
            .into_iter()
            .filter(|course| term.is_empty() || text(course.get("termLabel")) == term)
            .collect();
        let (lists, mut failed) = self.activity_lists(&courses).await;
        let mut owners = HashMap::new();
        let mut ids = Vec::new();
        for (course, items) in &lists {
            let Some(course_id) = course.get("id").and_then(Value::as_u64) else {
                continue;
            };
            for item in items {
                if text(item.get("type")) != "homework"
                    || item.get("published").and_then(Value::as_bool) == Some(false)
                {
                    continue;
                }
                if let Some(id) = item.get("id").and_then(Value::as_u64) {
                    ids.push(id);
                    owners.insert(id, course_id);
                }
            }
        }
        let (details, detail_failures) = self.details_of(&ids).await;
        failed += detail_failures;
        let mut assignments: HashMap<u64, Vec<Value>> = HashMap::new();
        for detail in details {
            let Some(id) = detail.get("id").and_then(Value::as_u64) else {
                continue;
            };
            let Some(course_id) = owners.get(&id).copied() else {
                continue;
            };
            assignments.entry(course_id).or_default().push(json!({
                "id": id,
                "title": detail["title"],
                "start": detail["start"],
                "end": detail["end"],
                "closed": detail["closed"],
                "submitCount": detail["submitCount"],
                "submitByGroup": detail["submitByGroup"],
            }));
        }
        let mut out = Vec::new();
        for course in courses {
            let Some(id) = course.get("id").and_then(Value::as_u64) else {
                continue;
            };
            let mut list = assignments.remove(&id).unwrap_or_default();
            list.sort_by(|left, right| {
                let left_end = left["end"].as_str().unwrap_or("");
                let right_end = right["end"].as_str().unwrap_or("");
                match (left_end.is_empty(), right_end.is_empty()) {
                    (true, true) => std::cmp::Ordering::Equal,
                    (true, false) => std::cmp::Ordering::Greater,
                    (false, true) => std::cmp::Ordering::Less,
                    _ => left_end.cmp(right_end),
                }
            });
            out.push(json!({
                "id": id,
                "name": course["name"],
                "code": course["code"],
                "subjectCode": course["subjectCode"],
                "term": term,
                "start": course["start"],
                "end": course["end"],
                "assignments": list,
            }));
        }
        Ok(json!({
            "term": term,
            "courses": out,
            "failed": failed,
        }))
    }

    async fn current_courses(&self) -> Result<Vec<Value>, String> {
        let value = self.courses().await?;
        let courses = value["courses"].as_array().cloned().unwrap_or_default();
        Ok(current_course_group(&courses, &campus_today()))
    }

    /// Fetch activity lists for a few courses at a time. The LMS is happiest
    /// with a small number of parallel requests, and one bad course must not
    /// sink the whole aggregation.
    async fn activity_lists(&self, courses: &[Value]) -> (Vec<(Value, Vec<Value>)>, usize) {
        let mut result = Vec::new();
        let mut failed = 0;
        for chunk in courses.chunks(6) {
            let mut tasks = Vec::new();
            for course in chunk {
                let Some(id) = course.get("id").and_then(Value::as_u64) else {
                    continue;
                };
                let client = self.clone();
                tasks.push((
                    course.clone(),
                    tokio::spawn(async move {
                        client.activities(id).await.map(|value| {
                            value["activities"].as_array().cloned().unwrap_or_default()
                        })
                    }),
                ));
            }
            for (course, task) in tasks {
                match task.await {
                    Ok(Ok(items)) => result.push((course, items)),
                    _ => failed += 1,
                }
            }
        }
        (result, failed)
    }

    /// Read activity details for homework/material rows, a few at a time.
    async fn details_of(&self, ids: &[u64]) -> (Vec<Value>, usize) {
        let mut result = Vec::new();
        let mut failed = 0;
        for chunk in ids.chunks(6) {
            let mut tasks = Vec::new();
            for id in chunk {
                let client = self.clone();
                let id = *id;
                tasks.push(tokio::spawn(async move { client.activity(id).await }));
            }
            for task in tasks {
                match task.await {
                    Ok(Ok(detail)) => result.push(detail),
                    _ => failed += 1,
                }
            }
        }
        (result, failed)
    }

    async fn player(&self, id: u64) -> Result<String, String> {
        let value = self
            .get(&format!("/api/lessons/{id}/player-url?from_page=course"))
            .await?;
        value
            .get("url")
            .and_then(Value::as_str)
            .and_then(player_url)
            .ok_or_else(|| "回放地址不受支持".into())
    }

    /// Resolve a blob to the media URL that can be opened or downloaded without
    /// the LMS cookies.
    async fn file_url(&self, upload_id: u64) -> Result<Url, String> {
        if !(1..=9_999_999_999).contains(&upload_id) {
            return Err("附件编号无效".into());
        }
        let address =
            Transport::address(&url(&format!("/api/uploads/{upload_id}/blob"))?, self.mode)?;
        let response = self
            .transport
            .client
            .get(address)
            .send()
            .await
            .map_err(|_| "无法获取附件，请检查网络后重试")?;
        if response.status().as_u16() != 302 {
            return Err("附件暂时无法打开".into());
        }
        let location = response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .ok_or("附件地址缺失")?;
        media_url(location, self.mode)
    }

    pub async fn open_file(&self, upload_id: u64) -> Result<Url, String> {
        self.file_url(upload_id).await
    }

    /// Stream an attachment to `path` and return the number of bytes written.
    /// Bytes go to a sibling `.part` file and are renamed only on success, so a
    /// broken download never leaves a truncated file where the user expects it.
    pub async fn save_file(&self, upload_id: u64, path: &Path) -> Result<u64, String> {
        let media = self.file_url(upload_id).await?;
        // Keep the session cookies: through WebVPN the mapped media URL is
        // only served to an authenticated portal session.
        let client = reqwest::Client::builder()
            .cookie_provider(self.transport.cookies.clone())
            .user_agent(crate::network::USER_AGENT)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|_| "无法创建附件连接")?;
        let mut response = client
            .get(media)
            .send()
            .await
            .map_err(|_| "无法连接附件服务器，请检查网络后重试")?;
        if !response.status().is_success() {
            return Err(format!(
                "附件服务器返回 HTTP {}",
                response.status().as_u16()
            ));
        }
        if response
            .content_length()
            .is_some_and(|len| len > MAX_DOWNLOAD)
        {
            return Err("附件超过 2 GB，未下载".into());
        }
        let temp = path.with_file_name(format!(
            ".{}.part",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("附件")
        ));
        let written = async {
            let mut file = tokio::fs::File::create(&temp)
                .await
                .map_err(|_| "无法写入所选位置")?;
            use tokio::io::AsyncWriteExt;
            let mut written = 0u64;
            while let Some(chunk) = response.chunk().await.map_err(|_| "附件下载中断")? {
                written += chunk.len() as u64;
                if written > MAX_DOWNLOAD {
                    return Err("附件超过 2 GB，已停止下载".to_string());
                }
                file.write_all(&chunk).await.map_err(|_| "写入附件失败")?;
            }
            file.flush().await.map_err(|_| "写入附件失败")?;
            Ok::<u64, String>(written)
        }
        .await;
        match written {
            Ok(written) => {
                // Windows refuses to rename over an existing file; the user
                // already agreed to replace it in the save dialog.
                if tokio::fs::rename(&temp, path).await.is_err() {
                    if tokio::fs::remove_file(path).await.is_err() {
                        let _ = tokio::fs::remove_file(&temp).await;
                        return Err("无法保存附件".into());
                    }
                    tokio::fs::rename(&temp, path)
                        .await
                        .map_err(|_| "无法保存附件".to_string())?;
                }
                Ok(written)
            }
            Err(error) => {
                let _ = tokio::fs::remove_file(&temp).await;
                Err(error)
            }
        }
    }

    /// Validate a submission without touching files or the school: the Agent
    /// should show this to the user before asking for confirmation.
    pub async fn homework_preview(&self, request: &Homework) -> Result<Value, String> {
        request.validate()?;
        let value = self
            .get(&format!("/api/activities/{}", request.activity))
            .await?;
        if text(value.get("type")) != "homework" {
            return Err("该活动不是作业，不能提交".into());
        }
        let files = resolve_files(&request.files).await?;
        let total: u64 = files.iter().map(|file| file.size).sum();
        Ok(json!({
            "confirmRequired": true,
            "activity": request.activity,
            "title": text(value.get("title")),
            "courseId": value.get("course_id").and_then(Value::as_u64),
            "start": campus_time(value.get("start_time")),
            "end": campus_time(value.get("end_time")),
            "closed": value.get("is_closed").and_then(Value::as_bool).unwrap_or(false),
            "resubmitOpen": value.get("is_resubmit_open").and_then(Value::as_bool).unwrap_or(false),
            "canSubmit": !submission_blocked(&value, request.draft),
            "submitCount": value.get("user_submit_count").and_then(Value::as_u64),
            "draft": request.draft,
            "comment": request.comment.trim(),
            "files": files.iter().map(|file| json!({
                "name": file.name,
                "size": file.size,
            })).collect::<Vec<_>>(),
            "totalSize": total,
            "note": "预览阶段没有上传或提交。请向用户复述课程、作业、文件和文字；用户明确确认后再带 confirm=true 调用。",
        }))
    }

    /// Upload the attachments and submit once. Any error that leaves the
    /// server's answer unknown is reported as such and never retried.
    pub async fn homework_submit(&self, request: &Homework) -> Result<Value, String> {
        request.validate()?;
        let value = self
            .get(&format!("/api/activities/{}", request.activity))
            .await?;
        let title = text(value.get("title"));
        if text(value.get("type")) != "homework" {
            return Err("该活动不是作业，不能提交".into());
        }
        // Refuse before uploading so no attachment is left orphaned on the
        // school side when the activity cannot accept a submission.
        if submission_blocked(&value, request.draft) {
            return Err("作业已截止，思源学堂当前不允许补交；本次没有上传或提交".into());
        }
        let files = resolve_files(&request.files).await?;
        let mode = text(value.pointer("/data/mode"));
        let mut uploads = Vec::new();
        for file in &files {
            match self.upload(file).await {
                Ok(id) => uploads.push(id),
                Err(error) => {
                    for id in &uploads {
                        self.delete_upload(*id).await;
                    }
                    return Err(error);
                }
            }
        }
        let body = json!({
            "comment": request.comment.trim(),
            "uploads": uploads,
            "slides": [],
            "is_draft": request.draft,
            "mode": if mode.is_empty() { "normal" } else { &mode },
            "other_resources": [],
            "uploads_in_rich_text": [],
        });
        let address = Transport::address(
            &url(&format!(
                "/api/course/activities/{}/submissions",
                request.activity
            ))?,
            self.mode,
        )?;
        let response = self
            .transport
            .client
            .post(address)
            .header("Referer", format!("{BASE}/"))
            .header("Accept", "application/json")
            .header("X-Requested-With", "XMLHttpRequest")
            .json(&body)
            .send()
            .await;
        let response = match response {
            Ok(response) => response,
            Err(_) => {
                return Err("提交结果未知：网络中断。请先到思源学堂查看该作业是否已经提交，确认前不要重复提交。".into())
            }
        };
        let status = response.status();
        if status.is_redirection() || matches!(status.as_u16(), 401 | 403) {
            return Err("思源学堂登录已失效，请刷新后重试；本次作业尚未确认提交".into());
        }
        let bytes =
            match response.bytes().await {
                Ok(bytes) => bytes,
                Err(_) => return Err(
                    "提交结果未知：读取响应时网络中断。请先到思源学堂确认，确认前不要重复提交。"
                        .into(),
                ),
            };
        let answer: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        // A success status with an unexpected body (for example an HTML login
        // page) cannot prove the submission exists, so report it as unknown.
        if status.is_success() && !answer.is_object() {
            return Err(
                "提交结果未知：思源学堂返回了无法识别的内容。请先到思源学堂查看，确认前不要重复提交。"
                    .into(),
            );
        }
        if !status.is_success() {
            if status.is_client_error() {
                // The school refused the request, so the uploaded files are
                // orphaned; remove them without retrying the submission.
                for id in &uploads {
                    self.delete_upload(*id).await;
                }
                let reason = server_reason(&answer);
                return Err(if reason.is_empty() {
                    format!("思源学堂拒绝提交（HTTP {}）", status.as_u16())
                } else {
                    format!("思源学堂拒绝提交：{reason}")
                });
            }
            return Err(format!(
                "提交结果未知：思源学堂返回 HTTP {}。请先到思源学堂查看，确认前不要重复提交。",
                status.as_u16()
            ));
        }
        let submission = answer.get("submission");
        let message = text(answer.get("message"));
        Ok(json!({
            "activity": request.activity,
            "title": title,
            "draft": request.draft,
            "submissionId": submission.and_then(|item| item.get("id")).and_then(Value::as_u64),
            "submittedAt": submission.map(|item| campus_time(item.get("submit_time"))).unwrap_or_default(),
            "files": files.iter().map(|file| json!({
                "name": file.name,
                "size": file.size,
            })).collect::<Vec<_>>(),
            "message": if message.is_empty() {
                if request.draft { "草稿已保存" } else { "作业已提交" }.to_string()
            } else {
                message
            },
        }))
    }

    async fn upload(&self, file: &UploadFile) -> Result<u64, String> {
        let metadata = self
            .post_json(
                "/api/uploads",
                &json!({
                    "name": file.name,
                    "size": file.size,
                    "parent_type": null,
                    "parent_id": 0,
                    "is_scorm": false,
                    "is_wmpkg": false,
                    "source": "",
                    "is_marked_attachment": false,
                    "embed_material_type": "",
                }),
            )
            .await?;
        let id = metadata
            .get("id")
            .and_then(Value::as_u64)
            .filter(|id| *id > 0)
            .ok_or("思源学堂没有返回上传编号")?;
        let storage = text(metadata.get("storage_type"));
        if !matches!(storage.as_str(), "MEDIA" | "LOCAL" | "") {
            self.delete_upload(id).await;
            return Err(format!("思源学堂使用暂不支持的存储方式（{storage}）"));
        }
        let raw = metadata
            .get("upload_url")
            .and_then(Value::as_str)
            .ok_or("思源学堂没有返回上传地址")?;
        let address = match upload_url(raw, id) {
            Ok(address) => address,
            Err(error) => {
                self.delete_upload(id).await;
                return Err(error);
            }
        };
        let part = match reqwest::multipart::Part::file(&file.path).await {
            Ok(part) => part.file_name(file.name.clone()),
            Err(_) => {
                self.delete_upload(id).await;
                return Err("无法读取要上传的文件".into());
            }
        };
        let form = reqwest::multipart::Form::new().part("file", part);
        // The transport's 25-second budget is for small API calls; a large
        // attachment needs more time.
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(600))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(crate::network::USER_AGENT)
            .build()
        {
            Ok(client) => client,
            Err(_) => {
                self.delete_upload(id).await;
                return Err("无法创建上传连接".into());
            }
        };
        let response = client
            .put(address)
            .header("Referer", format!("{BASE}/"))
            .multipart(form)
            .send()
            .await;
        match response {
            Ok(response) if response.status().is_success() => Ok(id),
            Ok(response) => {
                let status = response.status();
                self.delete_upload(id).await;
                Err(format!("文件上传失败（HTTP {}）", status.as_u16()))
            }
            Err(_) => {
                self.delete_upload(id).await;
                Err("文件上传中断，已取消本次上传".into())
            }
        }
    }

    /// Best-effort cleanup; a failed delete must not hide the original error.
    async fn delete_upload(&self, id: u64) {
        let Ok(path) = url(&format!("/api/uploads/{id}")) else {
            return;
        };
        let Ok(address) = Transport::address(&path, self.mode) else {
            return;
        };
        let _ = self
            .transport
            .client
            .delete(address)
            .header("Referer", format!("{BASE}/"))
            .header("Accept", "application/json")
            .header("X-Requested-With", "XMLHttpRequest")
            .send()
            .await;
    }
}

/// The school hands back an absolute upload URL on the media host. Only the
/// exact origin and path shape observed from the site are accepted.
fn upload_url(raw: &str, id: u64) -> Result<Url, String> {
    let address = Url::parse(raw).map_err(|_| "思源学堂返回的上传地址无效")?;
    if address.scheme() != "https"
        || address.host_str() != Some(MEDIA_HOST)
        || address.port_or_known_default() != Some(443)
        || !address.username().is_empty()
        || address.password().is_some()
        || address.path() != format!("/upload/file/{id}")
        || address.fragment().is_some()
    {
        return Err("思源学堂返回的上传地址不受支持".into());
    }
    Ok(address)
}

/// File types and locations that must never be uploaded through the Agent.
fn file_policy(path: &Path) -> Result<(), String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("文件名无效")?;
    if name.is_empty() || name.chars().count() > 200 || name.chars().any(char::is_control) {
        return Err(format!("文件名无效：{name}"));
    }
    for component in path.components() {
        if let Component::Normal(part) = component {
            if matches!(
                part.to_str().unwrap_or_default(),
                ".ssh"
                    | ".gnupg"
                    | ".xjtu-launcher"
                    | ".aws"
                    | ".kube"
                    | ".docker"
                    | ".password-store"
                    | "Keychains"
            ) {
                return Err("出于安全考虑，不能上传系统凭据目录中的文件".into());
            }
        }
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !UPLOAD_EXTENSIONS.contains(&extension.as_str()) {
        return Err(format!("不支持上传此类型的文件：{name}"));
    }
    Ok(())
}

async fn resolve_files(paths: &[String]) -> Result<Vec<UploadFile>, String> {
    let mut files = Vec::new();
    let mut total = 0u64;
    for path in paths {
        let raw = Path::new(path);
        if !raw.is_absolute() {
            return Err(format!("请使用绝对路径：{path}"));
        }
        let link = tokio::fs::symlink_metadata(raw)
            .await
            .map_err(|_| format!("找不到文件：{path}"))?;
        if link.file_type().is_symlink() {
            return Err(format!("出于安全考虑，不能上传符号链接：{path}"));
        }
        if !link.is_file() {
            return Err(format!("只能上传普通文件：{path}"));
        }
        let canonical = tokio::fs::canonicalize(raw)
            .await
            .map_err(|_| format!("无法读取文件：{path}"))?;
        file_policy(&canonical)?;
        let metadata = tokio::fs::metadata(&canonical)
            .await
            .map_err(|_| format!("无法读取文件：{path}"))?;
        let size = metadata.len();
        if size == 0 {
            return Err(format!("文件为空：{path}"));
        }
        if size > MAX_UPLOAD {
            return Err(format!("单个文件不能超过 100 MB：{path}"));
        }
        total = total.checked_add(size).ok_or("文件总大小无效")?;
        if total > MAX_UPLOAD_TOTAL {
            return Err("一次上传的文件总大小不能超过 200 MB".into());
        }
        let name = canonical
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("文件名无效")?
            .to_string();
        files.push(UploadFile {
            path: canonical,
            name,
            size,
        });
    }
    Ok(files)
}

/// Pull a short human-readable reason out of the school's error payload.
fn server_reason(value: &Value) -> String {
    let mut parts = Vec::new();
    for key in ["message", "error"] {
        let part = text(value.get(key));
        if !part.is_empty() {
            parts.push(part);
        }
    }
    if let Some(errors) = value.get("errors").and_then(Value::as_object) {
        for item in errors.values().take(3) {
            match item {
                Value::String(part) => parts.push(part.clone()),
                Value::Array(items) => {
                    if let Some(part) = items.iter().filter_map(Value::as_str).next() {
                        parts.push(part.to_string());
                    }
                }
                _ => {}
            }
        }
    }
    parts.join("；").chars().take(300).collect()
}

fn course(value: &Value) -> Option<Value> {
    let id = value.get("id")?.as_i64()?;
    let year = text(value.pointer("/academic_year/name"));
    let term = text(value.pointer("/semester/real_name"));
    let term = if term.is_empty() {
        text(value.pointer("/semester/name"))
    } else {
        term
    };
    let instructors = value
        .get("instructors")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .map(|item| text(item.get("name")))
                .filter(|name| !name.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(json!({
        "id": id,
        "name": text(value.get("name")),
        "code": text(value.get("course_code")),
        "subjectCode": text(value.get("subject_code")),
        "type": text(value.get("course_type")),
        "termLabel": course_term(value),
        "credit": value.get("credit").and_then(Value::as_f64),
        "class": text(value.get("klass")),
        "department": text(value.pointer("/department/name")),
        "instructors": instructors,
        "year": year,
        "term": term,
        "start": text(value.get("start_date")),
        "end": text(value.get("end_date")),
        "published": value.pointer("/course_attributes/published").and_then(Value::as_bool).unwrap_or(true),
    }))
}

/// The label shown for a course's semester: the derived term when we have one,
/// otherwise whatever year/term the school did fill in.
fn course_semester_label(course: &Value) -> String {
    let label = text(course.get("termLabel"));
    if !label.is_empty() {
        return label;
    }
    [text(course.get("year")), text(course.get("term"))]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The semester a course belongs to, as a folder-shaped label
/// (`2026-2027 秋季学期`). 思源学堂 usually leaves `semester.name` empty, but
/// the prefix of `course_code` is the semester code (`202620271…` = first
/// term). When that is missing the start date decides.
fn course_term(value: &Value) -> String {
    let start = text(value.get("start_date"));
    let year = text(value.pointer("/academic_year/name"));
    let code = text(value.get("course_code"));
    let semester = text(value.pointer("/semester/code"));
    let mut ordinal = 0u8;
    let digits: String = year.chars().filter(char::is_ascii_digit).collect();
    if digits.len() == 8 && code.starts_with(&digits) {
        ordinal = code.as_bytes()[8].wrapping_sub(b'0');
    }
    if !(1..=3).contains(&ordinal) && semester.len() >= 9 {
        ordinal = semester
            .as_bytes()
            .last()
            .map(|byte| byte.wrapping_sub(b'0'))
            .unwrap_or(0);
    }
    let month = start.get(5..7).and_then(|part| part.parse::<u8>().ok());
    let start_year = start.get(..4).and_then(|part| part.parse::<i32>().ok());
    if !(1..=3).contains(&ordinal) {
        ordinal = match month {
            Some(8..=12) | Some(1) => 1,
            Some(2..=7) => 2,
            _ => 0,
        };
    }
    let year = if digits.len() == 8 {
        format!("{}-{}", &digits[..4], &digits[4..])
    } else if let Some(year) = start_year {
        match month {
            Some(1) => format!("{}-{}", year - 1, year),
            Some(month) if month >= 8 => format!("{year}-{}", year + 1),
            Some(_) => format!("{}-{year}", year - 1),
            None => String::new(),
        }
    } else {
        String::new()
    };
    let season = match ordinal {
        1 => "秋季",
        2 => "春季",
        3 => "夏季",
        _ => return String::new(),
    };
    if year.is_empty() {
        String::new()
    } else {
        format!("{year} {season}学期")
    }
}

fn activity(value: &Value) -> Option<Value> {
    let id = value.get("id")?.as_i64()?;
    Some(json!({
        "id": id,
        "type": text(value.get("type")),
        "title": text(value.get("title")),
        "start": campus_time(value.get("start_time")),
        "end": campus_time(value.get("end_time")),
        "published": value.get("published").and_then(Value::as_bool).unwrap_or(true),
        "group": value.get("submit_by_group").and_then(Value::as_bool).unwrap_or(false),
    }))
}

fn submission(value: &Value) -> Option<Value> {
    let id = value.get("id")?.as_u64()?;
    if id == 0 {
        return None;
    }
    let files = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(upload).collect::<Vec<_>>())
            .unwrap_or_default()
    };
    let correction = value
        .get("submission_correct")
        .filter(|item| item.is_object())
        .map(|item| {
            json!({
                "score": number(item.get("score")),
                "instructorScore": number(item.get("instructor_score")),
                "comment": plain_text(&text(item.get("comment"))),
                "updatedAt": campus_time(item.get("updated_at")),
                "files": item
                    .get("uploads")
                    .and_then(Value::as_array)
                    .map(|items| items.iter().filter_map(upload).collect::<Vec<_>>())
                    .unwrap_or_default(),
            })
        });
    Some(json!({
        "id": id,
        "status": text(value.get("status")),
        "score": number(value.get("score")),
        "rubricScore": number(value.get("rubric_score")),
        "submittedAt": campus_time(value.get("submitted_at")),
        "updatedAt": campus_time(value.get("updated_at")),
        "createdAt": campus_time(value.get("created_at")),
        "latest": value.get("is_latest_version").and_then(Value::as_bool).unwrap_or(false),
        "resubmitted": value.get("is_resubmitted").and_then(Value::as_bool).unwrap_or(false),
        "canRetract": value.get("can_retract").and_then(Value::as_bool).unwrap_or(false),
        "mode": text(value.get("mode")),
        "comment": plain_text(&text(value.get("comment"))),
        "instructorComment": plain_text(&text(value.get("instructor_comment"))),
        "files": files("uploads"),
        "correction": correction,
    }))
}

fn upload(value: &Value) -> Option<Value> {
    let id = value.get("id")?.as_u64()?;
    if id == 0 {
        return None;
    }
    Some(json!({
        "id": id,
        "name": text(value.get("name")),
        "size": value.get("size").and_then(Value::as_u64),
        "type": text(value.get("type")),
        "allowDownload": value.get("allow_download").and_then(Value::as_bool).unwrap_or(false),
    }))
}

fn replay_code(value: &Value) -> String {
    value
        .pointer("/lesson_resource/properties/replay_code")
        .or_else(|| value.pointer("/data/external_live_detail/replay_id"))
        .map(|code| text(Some(code)))
        .unwrap_or_default()
}

fn activity_detail(value: &Value) -> Value {
    let id = value.get("id").and_then(Value::as_u64).unwrap_or(0);
    let kind = text(value.get("type"));
    let data = value.get("data").cloned().unwrap_or(Value::Null);
    let external = data.get("external_live_detail");
    let uploads = value
        .get("uploads")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(upload).collect::<Vec<_>>())
        .unwrap_or_default();
    json!({
        "id": id,
        "courseId": value.get("course_id").and_then(Value::as_u64),
        "type": kind,
        "title": text(value.get("title")),
        "start": campus_time(value.get("start_time")),
        "end": campus_time(value.get("end_time")),
        "published": value.get("published").and_then(Value::as_bool).unwrap_or(true),
        "closed": value.get("is_closed").and_then(Value::as_bool).unwrap_or(false),
        "resubmitOpen": value.get("is_resubmit_open").and_then(Value::as_bool).unwrap_or(false),
        "allowRetract": data.get("allow_retract").and_then(Value::as_bool).unwrap_or(false),
        "mode": text(data.get("mode")),
        "description": plain_text(&text(data.get("description"))),
        "uploads": uploads,
        "submitByGroup": value.get("submit_by_group").and_then(Value::as_bool).unwrap_or(false),
        "submitCount": value.get("user_submit_count").and_then(Value::as_u64),
        "submitTimes": value.get("submit_times").and_then(Value::as_u64),
        "averageScore": number(value.get("average_score")),
        "highestScore": number(value.get("highest_score")),
        "lowestScore": number(value.get("lowest_score")),
        "scoredCount": value.get("has_score_count").and_then(Value::as_u64),
        "lessonStart": campus_time(data.get("lesson_start")),
        "lessonEnd": campus_time(data.get("lesson_end")),
        "duration": data.get("duration").and_then(Value::as_u64),
        "replayCode": replay_code(value),
        "liveName": text(external.and_then(|live| live.get("name"))),
        "liveStart": campus_time(external.and_then(|live| live.get("start_time"))),
        "liveEnd": campus_time(external.and_then(|live| live.get("end_time"))),
        "player": "",
    })
}

/// Descriptions arrive as HTML. The launcher shows them as text, so convert
/// block boundaries to newlines and drop the markup.
fn plain_text(html: &str) -> String {
    if html.is_empty() {
        return String::new();
    }
    let breaks = regex::Regex::new(r"(?i)<(br|/p|/div|/li|/tr|/h[1-6])\b[^>]*>").unwrap();
    let tags = regex::Regex::new(r"(?s)<[^>]*>").unwrap();
    let text = breaks.replace_all(html, "\n");
    let text = tags.replace_all(&text, "");
    let text = text
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    text.lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_times_read_as_campus_time() {
        let at = |raw: &str| campus_time(Some(&json!(raw)));
        assert_eq!(at("2026-10-30T15:59:00Z"), "2026-10-30 23:59:00");
        assert_eq!(at("2026-10-30T15:59:00.000Z"), "2026-10-30 23:59:00");
        assert_eq!(at("2026-12-31T16:00:00Z"), "2027-01-01 00:00:00");
        assert_eq!(at("2024-02-28T16:30:00Z"), "2024-02-29 00:30:00");
        assert_eq!(at("2026-10-30T23:59:00+08:00"), "2026-10-30 23:59:00");
        assert_eq!(at("2026-10-30T10:59:00-0500"), "2026-10-30 23:59:00");
        // Zoneless or unparsable values are kept as the school sent them.
        assert_eq!(at("2026-10-30 23:59:00"), "2026-10-30 23:59:00");
        assert_eq!(at("2026-09-01"), "2026-09-01");
        assert_eq!(at("2026-13-01T00:00:00Z"), "2026-13-01T00:00:00Z");
        assert_eq!(campus_time(None), "");
    }

    #[test]
    fn semester_labels_prefer_the_derived_term() {
        let derived = json!({"termLabel": "2026-2027 秋季学期", "year": "2026-2027", "term": ""});
        assert_eq!(course_semester_label(&derived), "2026-2027 秋季学期");
        let raw = json!({"termLabel": "", "year": "2026-2027", "term": "第一学期"});
        assert_eq!(course_semester_label(&raw), "2026-2027 · 第一学期");
        assert_eq!(course_semester_label(&json!({})), "");
    }

    #[test]
    fn closed_homework_refuses_before_uploading() {
        let open = json!({"is_closed": false, "is_resubmit_open": false});
        let closed = json!({"is_closed": true, "is_resubmit_open": false});
        let reopens = json!({"is_closed": true, "is_resubmit_open": true});
        assert!(!submission_blocked(&open, false));
        assert!(submission_blocked(&closed, false));
        assert!(!submission_blocked(&reopens, false));
        // Drafts may still be saved after the deadline.
        assert!(!submission_blocked(&closed, true));
        // Missing flags read as "not closed", never as a false refusal.
        assert!(!submission_blocked(&json!({}), false));
    }

    #[test]
    fn maps_courses_and_activities() {
        let mapped = course(&json!({
            "id": 12,
            "name": "操作系统",
            "course_code": "COMP3001",
            "course_type": "必修",
            "credit": 3.5,
            "instructors": [{"name": "王老师"}, {"name": ""}],
            "academic_year": {"name": "2025-2026"},
            "semester": {"name": "秋季", "real_name": "第一学期"},
            "department": {"name": "计算机学院"},
            "klass": "计算机2301",
            "course_attributes": {"published": true}
        }))
        .unwrap();
        assert_eq!(mapped["id"], 12);
        assert_eq!(mapped["term"], "第一学期");
        assert_eq!(mapped["instructors"], json!(["王老师"]));
        assert!(course(&json!({"name": "missing id"})).is_none());

        let brief = activity(&json!({
            "id": 3,
            "type": "homework",
            "title": "第一次作业",
            "end_time": "2026-10-30 23:59:00"
        }))
        .unwrap();
        assert_eq!(brief["type"], "homework");
        assert_eq!(brief["end"], "2026-10-30 23:59:00");
        assert!(activity(&json!({"title": "missing id"})).is_none());
    }

    #[test]
    fn details_keep_uploads_scores_and_replays() {
        let detail = activity_detail(&json!({
            "id": 7,
            "course_id": 12,
            "type": "homework",
            "title": "实验一",
            "published": true,
            "end_time": "2026-10-30 23:59:00",
            "user_submit_count": 1,
            "submit_times": 2,
            "average_score": 86,
            "highest_score": null,
            "lowest_score": "61",
            "has_score_count": 20,
            "data": {"description": "<p>请阅读<b>实验手册</b></p><p>第二段</p>"},
            "uploads": [
                {"id": 5, "name": "手册.pdf", "size": 1024, "type": "document", "allow_download": true},
                {"id": 0, "name": "no id"}
            ]
        }));
        assert_eq!(detail["description"], "请阅读实验手册\n第二段");
        assert_eq!(detail["uploads"].as_array().unwrap().len(), 1);
        assert_eq!(detail["uploads"][0]["allowDownload"], true);
        assert_eq!(detail["submitCount"], 1);
        assert_eq!(detail["averageScore"], 86.0);
        assert_eq!(detail["lowestScore"], 61.0);
        assert!(detail["highestScore"].is_null());

        let lesson = activity_detail(&json!({
            "id": 9,
            "type": "lesson",
            "lesson_resource": {"properties": {"replay_code": 12345}},
            "data": {"lesson_start": "2026-09-01 08:00", "lesson_end": "2026-09-01 09:40", "duration": 6000}
        }));
        assert_eq!(lesson["replayCode"], "12345");
        assert_eq!(lesson["duration"], 6000);
    }

    #[test]
    fn plain_text_drops_markup_but_keeps_lines() {
        assert_eq!(plain_text(""), "");
        assert_eq!(
            plain_text("<div>第一行<br/>第二行&nbsp;&amp;尾</div>"),
            "第一行\n第二行 &尾"
        );
        assert_eq!(plain_text("<img src='x'>文字"), "文字");
    }

    #[test]
    fn submissions_are_flattened_for_the_agent() {
        let value = json!({
            "list": [
                {
                    "id": 7,
                    "status": "submitted",
                    "score": 88,
                    "comment": "<p>答案</p>",
                    "instructor_comment": "不错",
                    "submitted_at": "2026-10-01 10:00:00",
                    "uploads": [{"id": 3, "name": "a.pdf", "size": 12}],
                    "submission_correct": {
                        "score": 88,
                        "instructor_score": 90,
                        "comment": "<b>思路清晰</b>",
                        "uploads": []
                    }
                },
                {"id": 0}
            ]
        });
        let list: Vec<_> = value["list"]
            .as_array()
            .expect("list")
            .iter()
            .filter_map(submission)
            .collect();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["comment"], "答案");
        assert_eq!(list[0]["instructorComment"], "不错");
        assert_eq!(list[0]["score"].as_f64(), Some(88.0));
        assert_eq!(list[0]["files"][0]["name"], "a.pdf");
        assert_eq!(list[0]["correction"]["comment"], "思路清晰");
        assert_eq!(
            list[0]["correction"]["instructorScore"].as_f64(),
            Some(90.0)
        );
    }

    #[test]
    fn homework_requests_need_content_and_sane_limits() {
        let mut request = Homework {
            activity: 12,
            comment: "  ".into(),
            files: Vec::new(),
            draft: false,
        };
        assert!(request.validate().is_err());
        request.comment = "答案".into();
        assert!(request.validate().is_ok());
        request.activity = 0;
        assert!(request.validate().is_err());
        request.activity = 12;
        request.files = vec!["/tmp/a.pdf".into(); MAX_UPLOAD_FILES + 1];
        assert!(request.validate().is_err());
        request.files.clear();
        request.comment = "x".repeat(MAX_COMMENT + 1);
        assert!(request.validate().is_err());
    }

    #[test]
    fn upload_addresses_are_limited_to_the_media_host() {
        for good in [
            "https://media.xjtu.edu.cn/upload/file/7678010",
            // The live site signs the upload with query parameters.
            "https://media.xjtu.edu.cn/upload/file/7678010?token=x&expires=1",
        ] {
            assert!(upload_url(good, 7_678_010).is_ok(), "{good}");
        }
        for bad in [
            "http://media.xjtu.edu.cn/upload/file/7678010",
            "https://media.xjtu.edu.cn.evil.test/upload/file/7678010",
            "https://u:p@media.xjtu.edu.cn/upload/file/7678010",
            "https://media.xjtu.edu.cn/upload/file/1",
            "https://media.xjtu.edu.cn/upload/file/7678010#fragment",
            "https://lms.xjtu.edu.cn/upload/file/7678010",
        ] {
            assert!(upload_url(bad, 7_678_010).is_err(), "{bad}");
        }
    }

    #[test]
    fn file_policy_blocks_credentials_and_unknown_types() {
        for good in [
            "/tmp/homework.pdf",
            "/Users/student/Documents/report.DOCX",
            "/tmp/实验一.zip",
            "/tmp/main.rs",
        ] {
            assert!(file_policy(Path::new(good)).is_ok(), "{good}");
        }
        for bad in [
            "/Users/student/.ssh/id_ed25519",
            "/Users/student/.ssh/id_ed25519.pub",
            "/Users/student/.xjtu-launcher/auth.json",
            "/tmp/auth.json",
            "/tmp/.env",
            "/tmp/secret.pem",
            "/tmp/private.key",
            "/tmp/keychain.json",
        ] {
            assert!(file_policy(Path::new(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn server_reasons_are_short_and_useful() {
        assert_eq!(
            server_reason(&json!({"message": "作业已截止"})),
            "作业已截止"
        );
        assert_eq!(
            server_reason(&json!({"errors": {"uploads": ["文件类型不允许"]}})),
            "文件类型不允许"
        );
        assert!(
            server_reason(&json!({"errors": {"base": ["x".repeat(1000)]}}))
                .chars()
                .count()
                <= 300
        );
        assert!(server_reason(&json!({})).is_empty());
    }

    /// Manual probe: compares the school's official todo list with the
    /// per-course overview, to find stale or incorrectly filtered homework.
    #[tokio::test]
    #[ignore = "live probe; reads the local account file and never prints secrets"]
    async fn live_todos_probe() {
        let home = std::env::var("HOME").expect("home");
        let directory = std::path::PathBuf::from(home).join(".xjtu-launcher");
        crate::vault::initialize(directory.clone()).expect("credential store");
        let book = crate::accounts::load(&directory.join("auth.json")).expect("account file");
        let account = book.active().expect("active account");
        let Some(saved) = account.password.as_ref() else {
            println!("no saved password");
            return;
        };
        let cookies = match crate::auth::direct_login(
            LOGIN_URL,
            &account.username,
            &saved.0,
            "live-todos-probe",
        )
        .await
        {
            Ok(cookies) => cookies,
            Err(error) => {
                println!("login error: {error}");
                return;
            }
        };
        let client = Client::with_cookies(&cookies, AccessMode::Direct).expect("client");
        match client.get("/api/todos").await {
            Ok(value) => {
                println!(
                    "todos keys={:?}",
                    value
                        .as_object()
                        .map(|object| object.keys().cloned().collect::<Vec<_>>())
                );
                let list = value
                    .get("todos")
                    .or_else(|| value.get("data"))
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                println!("todos count={}", list.len());
                for item in list.iter().take(30) {
                    println!(
                        "  id={} course={} title={} end={}",
                        item.get("id").and_then(Value::as_u64).unwrap_or(0),
                        item.get("course_name")
                            .or_else(|| item.get("courseName"))
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                        item.get("title").and_then(Value::as_str).unwrap_or(""),
                        item.get("end_time")
                            .or_else(|| item.get("endTime"))
                            .and_then(Value::as_str)
                            .unwrap_or("")
                    );
                }
            }
            Err(error) => println!("todos error: {error}"),
        }
        match client.overview().await {
            Ok(value) => {
                let rows = value["rows"].as_array().cloned().unwrap_or_default();
                println!("overview count={}", rows.len());
                for item in rows.iter().take(30) {
                    println!(
                        "  id={} course={} title={} submitCount={}",
                        item["id"].as_u64().unwrap_or(0),
                        item["course"].as_str().unwrap_or(""),
                        item["title"].as_str().unwrap_or(""),
                        item["submitCount"].as_u64().unwrap_or(0)
                    );
                }
            }
            Err(error) => println!("overview error: {error}"),
        }
    }

    /// Manual probe: resolves one real replay without printing tokens or URLs.
    #[tokio::test]
    #[ignore = "live probe; reads the local account file and never prints secrets"]
    async fn live_replay_video_probe() {
        let home = std::env::var("HOME").expect("home");
        let directory = std::path::PathBuf::from(home).join(".xjtu-launcher");
        crate::vault::initialize(directory.clone()).expect("credential store");
        let book = crate::accounts::load(&directory.join("auth.json")).expect("account file");
        let account = book.active().expect("active account");
        let Some(saved) = account.password.as_ref() else {
            println!("no saved password");
            return;
        };
        let cookies = match crate::auth::direct_login(
            LOGIN_URL,
            &account.username,
            &saved.0,
            "live-replay-probe",
        )
        .await
        {
            Ok(cookies) => cookies,
            Err(error) => {
                println!("login error: {error}");
                return;
            }
        };
        let client = Client::with_cookies(&cookies, AccessMode::Direct).expect("client");
        // Probe one lesson by id: XJTU_PROBE_ACTIVITY=1291505.
        let only = std::env::var("XJTU_PROBE_ACTIVITY")
            .ok()
            .and_then(|value| value.parse::<u64>().ok());
        let courses = client.courses().await.expect("courses");
        let courses = courses["courses"].as_array().cloned().unwrap_or_default();
        for course in courses.iter().take(8) {
            let course_id = course["id"].as_u64().unwrap_or(0);
            let Ok(list) = client.activities(course_id).await else {
                continue;
            };
            for item in list["activities"].as_array().cloned().unwrap_or_default() {
                if item["type"] != "lesson" {
                    continue;
                }
                let id = item["id"].as_u64().unwrap_or(0);
                if only.is_some_and(|target| target != id) {
                    continue;
                }
                let Ok(detail) = client.activity(id).await else {
                    continue;
                };
                let replay = detail["replayCode"].as_str().unwrap_or("");
                if replay.is_empty() {
                    continue;
                }
                let player = detail["player"].as_str().unwrap_or("");
                if player.is_empty() {
                    continue;
                }
                println!(
                    "lesson {id} replay={replay} title={}",
                    detail["title"].as_str().unwrap_or("")
                );
                match client.replay_videos(id).await {
                    Ok(found) => {
                        println!("videos={}", found.videos.len());
                        for video in &found.videos {
                            let host = Url::parse(&video.download_url)
                                .ok()
                                .and_then(|url| url.host_str().map(str::to_owned))
                                .unwrap_or_default();
                            println!(
                                "  label={} size={} mute={} best={} host={host} signed={}",
                                video.label,
                                video.size,
                                video.mute,
                                video.best_audio,
                                video.download_url.contains("X-Amz-Signature"),
                            );
                        }
                        if let Some(video) = found.videos.first() {
                            if let Ok(parsed) = Url::parse(&video.download_url) {
                                let probe = reqwest::Client::builder()
                                    .timeout(std::time::Duration::from_secs(30))
                                    .build()
                                    .expect("probe client");
                                match probe
                                    .get(parsed)
                                    .header("Range", "bytes=0-65535")
                                    .send()
                                    .await
                                {
                                    Ok(response) => println!(
                                        "range status={} type={:?} ranges={:?} cors={:?}",
                                        response.status(),
                                        response
                                            .headers()
                                            .get("content-type")
                                            .and_then(|v| v.to_str().ok()),
                                        response
                                            .headers()
                                            .get("accept-ranges")
                                            .and_then(|v| v.to_str().ok()),
                                        response
                                            .headers()
                                            .get("access-control-allow-origin")
                                            .and_then(|v| v.to_str().ok())
                                    ),
                                    Err(error) => println!("range error: {error}"),
                                }
                            }
                        }
                        return;
                    }
                    Err(error) => println!("replay error: {error}"),
                }
            }
        }
        println!("no replay found");
    }

    #[test]
    fn media_and_player_urls_are_restricted() {
        let direct = "https://media.xjtu.edu.cn/download/file/abc";
        assert!(media_url(direct, AccessMode::Direct).is_ok());
        for bad in [
            "http://media.xjtu.edu.cn/download/file/abc",
            "https://media.xjtu.edu.cn.evil.test/file",
            "https://u:p@media.xjtu.edu.cn/file",
            "https://example.test/file",
        ] {
            assert!(media_url(bad, AccessMode::Direct).is_err(), "{bad}");
            assert!(media_url(bad, AccessMode::Webvpn).is_err(), "{bad}");
        }
        // Through WebVPN the plain redirect is mapped, and an already mapped
        // redirect is checked by decoding it back to the media host.
        let original = Url::parse(direct).unwrap();
        let mapped = network::vpn_url(&original).unwrap();
        for location in [direct, mapped.as_str()] {
            let resolved = media_url(location, AccessMode::Webvpn).unwrap();
            assert_eq!(resolved, mapped);
        }
        // A mapped URL for another host is still refused.
        let other = network::vpn_url(&Url::parse("https://lms.xjtu.edu.cn/a").unwrap()).unwrap();
        assert!(media_url(other.as_str(), AccessMode::Webvpn).is_err());
        assert!(player_url(
            "https://rms-v5.xjtu.edu.cn/embed/lms/lesson-activity/a/detail?token=t"
        )
        .is_some());
        assert!(player_url("https://example.test/player").is_none());
        assert!(player_url("http://rms-v5.xjtu.edu.cn/player").is_none());
    }

    #[test]
    fn replay_videos_stay_on_the_review_host() {
        let video = replay_video(&json!({
            "label": "INSTRUCTOR",
            "size": 960_000_000u64,
            "mute": false,
            "is_best_audio": true,
            "download_url": "https://review-class.xjtu.edu.cn/v/1.mp4?X-Amz-Signature=abc",
        }))
        .expect("accepted");
        assert_eq!(video.label, "INSTRUCTOR");
        assert_eq!(video.size, 960_000_000);
        assert!(video.best_audio);
        for bad in [
            json!({"download_url": "http://review-class.xjtu.edu.cn/a.mp4"}),
            json!({"download_url": "https://review-class.xjtu.edu.cn.evil.test/a.mp4"}),
            json!({"download_url": "https://u:p@review-class.xjtu.edu.cn/a.mp4"}),
            json!({"download_url": "https://example.test/a.mp4"}),
            json!({"download_url": "file:///etc/passwd"}),
            json!({"label": "missing url"}),
        ] {
            assert!(replay_video(&bad).is_none(), "{bad}");
        }
        let camel = replay_video(&json!({
            "downloadUrl": "https://review-class.xjtu.edu.cn/a.mp4",
            "isBestAudio": true,
            "name": "ENCODER",
        }))
        .expect("camel case accepted");
        assert_eq!(camel.label, "ENCODER");
        assert!(camel.best_audio);
    }

    /// Manual probe: builds a homework preview with a temporary text file. It
    /// never uploads or submits anything.
    #[tokio::test]
    #[ignore = "live probe; reads the local account file and never submits"]
    async fn live_lms_homework_preview_probe() {
        let home = std::env::var("HOME").expect("home");
        let directory = std::path::PathBuf::from(home).join(".xjtu-launcher");
        crate::vault::initialize(directory.clone()).expect("credential store");
        let book = crate::accounts::load(&directory.join("auth.json")).expect("account file");
        let account = book.active().expect("active account");
        let Some(saved) = account.password.as_ref() else {
            println!("no saved password");
            return;
        };
        let cookies = match crate::auth::direct_login(
            LOGIN_URL,
            &account.username,
            &saved.0,
            "live-lms-probe",
        )
        .await
        {
            Ok(cookies) => cookies,
            Err(error) => {
                println!("login error: {error}");
                return;
            }
        };
        let client = Client::with_cookies(&cookies, AccessMode::Direct).expect("client");
        let courses = client.courses().await.expect("courses");
        let courses = courses["courses"].as_array().cloned().unwrap_or_default();
        for course in &courses {
            let course_id = course["id"].as_u64().unwrap_or(0);
            let Ok(list) = client.activities(course_id).await else {
                continue;
            };
            for item in list["activities"].as_array().cloned().unwrap_or_default() {
                if item["type"] != "homework" {
                    continue;
                }
                let path = std::env::temp_dir().join("xjtu-launcher-preview-probe.txt");
                std::fs::write(&path, b"preview only").expect("probe file");
                let request = Homework {
                    activity: item["id"].as_u64().unwrap_or(0),
                    comment: "预览测试，不会提交".into(),
                    files: vec![path.to_string_lossy().to_string()],
                    draft: false,
                };
                let result = client.homework_preview(&request).await;
                let _ = std::fs::remove_file(&path);
                match result {
                    Ok(value) => println!(
                        "preview activity={} title={} files={} confirm={}",
                        value["activity"],
                        value["title"].as_str().unwrap_or(""),
                        value["files"].as_array().map(Vec::len).unwrap_or(0),
                        value["confirmRequired"]
                    ),
                    Err(error) => println!("preview error: {error}"),
                }
                return;
            }
        }
        println!("no homework activity found");
    }

    /// Manual probe: uploads a one-byte text file and deletes it right away. It
    /// never creates a submission.
    #[tokio::test]
    #[ignore = "live probe; writes a temporary upload to the school and removes it"]
    async fn live_lms_upload_probe() {
        let home = std::env::var("HOME").expect("home");
        let directory = std::path::PathBuf::from(home).join(".xjtu-launcher");
        crate::vault::initialize(directory.clone()).expect("credential store");
        let book = crate::accounts::load(&directory.join("auth.json")).expect("account file");
        let account = book.active().expect("active account");
        let Some(saved) = account.password.as_ref() else {
            println!("no saved password");
            return;
        };
        let cookies = match crate::auth::direct_login(
            LOGIN_URL,
            &account.username,
            &saved.0,
            "live-lms-probe",
        )
        .await
        {
            Ok(cookies) => cookies,
            Err(error) => {
                println!("login error: {error}");
                return;
            }
        };
        let client = Client::with_cookies(&cookies, AccessMode::Direct).expect("client");
        let path = std::env::temp_dir().join("xjtu-launcher-upload-probe.txt");
        std::fs::write(&path, b"probe").expect("probe file");
        let files = resolve_files(&[path.to_string_lossy().to_string()])
            .await
            .expect("resolve");
        let uploaded = client.upload(&files[0]).await;
        let _ = std::fs::remove_file(&path);
        match uploaded {
            Ok(id) => {
                println!("upload id={id}");
                match client.get(&format!("/api/uploads/{id}?preview=true")).await {
                    Ok(value) => println!(
                        "upload status={} name={} size={}",
                        text(value.get("status")),
                        text(value.get("name")),
                        value.get("size").and_then(Value::as_u64).unwrap_or(0)
                    ),
                    Err(error) => println!("upload lookup error: {error}"),
                }
                client.delete_upload(id).await;
                println!("deleted");
            }
            Err(error) => println!("upload error: {error}"),
        }
    }

    /// Manual probe: signs in with the saved account and reads only the course
    /// count. It prints no cookies or password material.
    #[tokio::test]
    #[ignore = "live probe; reads the local account file and never prints secrets"]
    async fn live_lms_session_probe() {
        let home = std::env::var("HOME").expect("home");
        let directory = std::path::PathBuf::from(home).join(".xjtu-launcher");
        crate::vault::initialize(directory.clone()).expect("credential store");
        let book = crate::accounts::load(&directory.join("auth.json")).expect("account file");
        let account = book.active().expect("active account");
        let Some(saved) = account.password.as_ref() else {
            println!("no saved password");
            return;
        };
        println!("mode={:?}", account.mode);
        match crate::auth::direct_login(LOGIN_URL, &account.username, &saved.0, "live-lms-probe")
            .await
        {
            Ok(cookies) => {
                let client = Client::with_cookies(&cookies, AccessMode::Direct).expect("client");
                match client.courses().await {
                    Ok(value) => {
                        let courses = value
                            .get("courses")
                            .and_then(Value::as_array)
                            .cloned()
                            .unwrap_or_default();
                        println!("courses={}", courses.len());
                        for course in courses.iter().take(3) {
                            let id = course.get("id").and_then(Value::as_u64).unwrap_or(0);
                            match client.activities(id).await {
                                Ok(value) => println!(
                                    "course {id} activities={}",
                                    value
                                        .get("activities")
                                        .and_then(Value::as_array)
                                        .map(Vec::len)
                                        .unwrap_or(0)
                                ),
                                Err(error) => println!("course {id} error: {error}"),
                            }
                        }
                    }
                    Err(error) => println!("courses error: {error}"),
                }
            }
            Err(error) => println!("login error: {error}"),
        }
    }

    /// Manual probe: discovers how to read the current user id and the
    /// submission list for one homework. It prints counts and status only.
    #[tokio::test]
    #[ignore = "live probe; reads the local account file and never prints secrets"]
    async fn live_lms_submissions_probe() {
        let home = std::env::var("HOME").expect("home");
        let directory = std::path::PathBuf::from(home).join(".xjtu-launcher");
        crate::vault::initialize(directory.clone()).expect("credential store");
        let book = crate::accounts::load(&directory.join("auth.json")).expect("account file");
        let account = book.active().expect("active account");
        let Some(saved) = account.password.as_ref() else {
            println!("no saved password");
            return;
        };
        let cookies = match crate::auth::direct_login(
            LOGIN_URL,
            &account.username,
            &saved.0,
            "live-lms-probe",
        )
        .await
        {
            Ok(cookies) => cookies,
            Err(error) => {
                println!("login error: {error}");
                return;
            }
        };
        let client = Client::with_cookies(&cookies, AccessMode::Direct).expect("client");
        let courses = client.courses().await.expect("courses");
        let courses = courses["courses"].as_array().cloned().unwrap_or_default();
        // Scan a few courses so a graded submission is likely to appear; print
        // only counts and field names, never submission text.
        for course in courses.iter().take(10) {
            let course_id = course["id"].as_u64().unwrap_or(0);
            let Ok(list) = client.activities(course_id).await else {
                continue;
            };
            for item in list["activities"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter(|item| item["type"] == "homework")
                .take(5)
            {
                let id = item["id"].as_u64().unwrap_or(0);
                match client.submissions(id).await {
                    Ok(value) => {
                        let count = value["submissions"].as_array().map(Vec::len).unwrap_or(0);
                        println!(
                            "homework {id} group={} submissions={count}",
                            value["submitByGroup"]
                        );
                        if count > 0 {
                            println!(
                                "first submission keys={:?}",
                                value["submissions"][0]
                                    .as_object()
                                    .map(|object| object.keys().cloned().collect::<Vec<_>>())
                            );
                            return;
                        }
                    }
                    Err(error) => println!("homework {id} error: {error}"),
                }
            }
        }
        println!("no submitted homework found");
    }

    /// Manual probe: checks activity details, the replay URL and one real
    /// attachment download. It prints no file contents or session material.
    #[tokio::test]
    #[ignore = "live probe; reads the local account file and never prints secrets"]
    async fn live_lms_activity_probe() {
        let home = std::env::var("HOME").expect("home");
        let directory = std::path::PathBuf::from(home).join(".xjtu-launcher");
        crate::vault::initialize(directory.clone()).expect("credential store");
        let book = crate::accounts::load(&directory.join("auth.json")).expect("account file");
        let account = book.active().expect("active account");
        let Some(saved) = account.password.as_ref() else {
            println!("no saved password");
            return;
        };
        let cookies = match crate::auth::direct_login(
            LOGIN_URL,
            &account.username,
            &saved.0,
            "live-lms-probe",
        )
        .await
        {
            Ok(cookies) => cookies,
            Err(error) => {
                println!("login error: {error}");
                return;
            }
        };
        let client = Client::with_cookies(&cookies, AccessMode::Direct).expect("client");
        let courses = client.courses().await.expect("courses");
        let courses = courses["courses"].as_array().cloned().unwrap_or_default();
        let mut seen = std::collections::BTreeSet::new();
        let mut sample = None;
        for course in &courses {
            let id = course["id"].as_u64().unwrap_or(0);
            let Ok(list) = client.activities(id).await else {
                continue;
            };
            for item in list["activities"].as_array().cloned().unwrap_or_default() {
                let kind = item["type"].as_str().unwrap_or("").to_string();
                if seen.contains(&kind) {
                    continue;
                }
                let id = item["id"].as_u64().unwrap_or(0);
                match client.activity(id).await {
                    Ok(detail) => {
                        println!(
                            "{kind}: uploads={} desc={} replay={} player={}",
                            detail["uploads"].as_array().map(Vec::len).unwrap_or(0),
                            detail["description"].as_str().map(str::len).unwrap_or(0),
                            !detail["replayCode"].as_str().unwrap_or("").is_empty(),
                            !detail["player"].as_str().unwrap_or("").is_empty()
                        );
                        if sample.is_none() {
                            sample = detail["uploads"]
                                .as_array()
                                .and_then(|list| list.first())
                                .and_then(|upload| upload["id"].as_u64());
                        }
                        seen.insert(kind);
                    }
                    Err(error) => println!("{kind}: detail error {error}"),
                }
            }
        }
        if let Some(upload) = sample {
            let path = std::env::temp_dir().join("xjtu-lms-probe.bin");
            match client.save_file(upload, &path).await {
                Ok(bytes) => println!("saved={bytes}"),
                Err(error) => println!("save error: {error}"),
            }
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn course_group_prefers_running_dates_then_the_newest_cohort() {
        let course = |id: u64, start: &str, end: &str| json!({"id": id, "start": start, "end": end, "year": "2026-2027"});
        let ids = |group: Vec<Value>| {
            let mut ids = group
                .iter()
                .filter_map(|course| course["id"].as_u64())
                .collect::<Vec<_>>();
            ids.sort_unstable();
            ids
        };
        let running = vec![
            course(1, "2026-09-01", "2027-01-15"),
            course(2, "2025-09-08", ""),
            course(3, "2026-12-01", "2027-03-01"),
        ];
        assert_eq!(ids(current_course_group(&running, "2026-10-31")), [1]);
        // An open-ended course from the same term is part of the running group.
        let mixed = vec![
            course(1, "2026-09-01", "2027-01-15"),
            course(2, "2026-09-14", ""),
            course(3, "2025-02-17", ""),
        ];
        assert_eq!(ids(current_course_group(&mixed, "2026-10-31")), [1, 2]);

        // An empty end date is common; the newest start cohort wins.
        let open_ended = vec![
            course(1, "2025-02-17", ""),
            course(2, "2026-09-14", ""),
            course(3, "2026-09-16", ""),
            course(4, "2026-03-02", ""),
        ];
        assert_eq!(ids(current_course_group(&open_ended, "2026-10-31")), [2, 3]);

        // Before the term starts, show the nearest upcoming cohort rather than
        // everything (or a cohort a whole year further out).
        let upcoming = vec![
            course(1, "2027-03-01", ""),
            course(2, "2027-03-03", ""),
            course(3, "2027-09-06", ""),
        ];
        assert_eq!(ids(current_course_group(&upcoming, "2026-10-31")), [1, 2]);

        // Without any dates the aggregate has no better signal than all courses.
        let undated = vec![course(1, "", ""), course(2, "", "")];
        assert_eq!(ids(current_course_group(&undated, "2026-10-31")), [1, 2]);
        assert_eq!(day_number("1970-01-01"), Some(0));
        assert_eq!(day_number("2026-09-14"), Some(20_710));
        assert_eq!(day_number("not-a-date"), None);
    }

    /// Manual probe: prints how courses are labelled and dated, so the
    /// current-semester rule can be checked without exposing course names.
    #[tokio::test]
    #[ignore = "live probe; reads the local account file and never prints secrets"]
    async fn live_lms_course_dates_probe() {
        let home = std::env::var("HOME").expect("home");
        let directory = std::path::PathBuf::from(home).join(".xjtu-launcher");
        crate::vault::initialize(directory.clone()).expect("credential store");
        let book = crate::accounts::load(&directory.join("auth.json")).expect("account file");
        let account = book.active().expect("active account");
        let Some(saved) = account.password.as_ref() else {
            println!("no saved password");
            return;
        };
        let Ok(cookies) =
            crate::auth::direct_login(LOGIN_URL, &account.username, &saved.0, "live-lms-probe")
                .await
        else {
            println!("login error");
            return;
        };
        let client = Client::with_cookies(&cookies, AccessMode::Direct).expect("client");
        let all = client.courses().await.expect("courses");
        let all = all["courses"].as_array().cloned().unwrap_or_default();
        let mut cohorts: HashMap<(String, String, String, String), usize> = HashMap::new();
        for course in &all {
            let key = (
                text(course.get("year")),
                text(course.get("term")),
                text(course.get("start")),
                text(course.get("end")),
            );
            *cohorts.entry(key).or_insert(0) += 1;
        }
        let mut cohorts = cohorts.into_iter().collect::<Vec<_>>();
        cohorts.sort();
        println!("today={} courses={}", campus_today(), all.len());
        for ((year, term, start, end), count) in cohorts {
            println!("year={year:?} term={term:?} start={start:?} end={end:?} count={count}");
        }
    }

    /// Manual probe: checks that the current-semester aggregation returns
    /// without scanning every course, and prints counts only.
    #[tokio::test]
    #[ignore = "live probe; reads the local account file and never prints secrets"]
    async fn live_lms_aggregate_probe() {
        let home = std::env::var("HOME").expect("home");
        let directory = std::path::PathBuf::from(home).join(".xjtu-launcher");
        crate::vault::initialize(directory.clone()).expect("credential store");
        let book = crate::accounts::load(&directory.join("auth.json")).expect("account file");
        let account = book.active().expect("active account");
        let Some(saved) = account.password.as_ref() else {
            println!("no saved password");
            return;
        };
        let cookies = match crate::auth::direct_login(
            LOGIN_URL,
            &account.username,
            &saved.0,
            "live-lms-probe",
        )
        .await
        {
            Ok(cookies) => cookies,
            Err(error) => {
                println!("login error: {error}");
                return;
            }
        };
        let client = Client::with_cookies(&cookies, AccessMode::Direct).expect("client");
        let all = client.courses().await.expect("courses");
        let all = all["courses"].as_array().cloned().unwrap_or_default();
        let today = campus_today();
        let current = current_course_group(&all, &today);
        println!(
            "today={today} courses={} current={}",
            all.len(),
            current.len()
        );
        for course in &current {
            println!(
                "current course id={} start={} end={}",
                course["id"], course["start"], course["end"]
            );
        }
        match client.overview().await {
            Ok(value) => {
                let files = value["files"].as_array().cloned().unwrap_or_default();
                let courses = files
                    .iter()
                    .filter_map(|file| file["courseId"].as_u64())
                    .collect::<HashSet<_>>()
                    .len();
                println!(
                    "semester={} todos={} library files={} courses={courses} failed={}",
                    value["semester"],
                    value["rows"].as_array().map(Vec::len).unwrap_or(0),
                    files.len(),
                    value["failed"]
                );
            }
            Err(error) => println!("overview error: {error}"),
        }
    }

    #[test]
    fn rejects_paths_outside_the_api() {
        assert!(url("/api/courses/1").is_ok());
        assert!(url("/user/index").is_err());
        assert!(url("/api/../admin").is_err());
    }
}
