//! 思源学堂 read-only adapter. The site keeps its own CAS cookie session, so
//! the account stores it under `sites.lms` and every call uses that jar.
use crate::{
    accounts::Account,
    network::{AccessMode, Transport},
};
use serde_json::{json, Value};
use std::{path::Path, time::Duration};
use url::Url;

const BASE: &str = "https://lms.xjtu.edu.cn";
const MEDIA_HOST: &str = "media.xjtu.edu.cn";
const PLAYER_HOST: &str = "rms-v5.xjtu.edu.cn";
/// Attachment and replay downloads are user-initiated; keep a hard ceiling so a
/// wrong link cannot fill the disk.
const MAX_DOWNLOAD: u64 = 2 * 1024 * 1024 * 1024;
pub(crate) const LOGIN_URL: &str = BASE;

pub struct Client {
    transport: Transport,
}

fn text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
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
fn media_url(location: &str) -> Result<Url, String> {
    let url = Url::parse(location).map_err(|_| "附件地址无效")?;
    if url.scheme() != "https"
        || url.host_str() != Some(MEDIA_HOST)
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("附件地址不受支持".into());
    }
    Ok(url)
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

impl Client {
    pub fn connect(account: &Account) -> Result<Self, String> {
        let site = account.sites.get("lms").ok_or("请先完成思源学堂登录")?;
        let cookies = site
            .cookies
            .as_deref()
            .filter(|cookies| !cookies.is_empty())
            .ok_or("请先完成思源学堂登录")?;
        Self::with_cookies(cookies)
    }

    fn with_cookies(cookies: &str) -> Result<Self, String> {
        Ok(Self {
            transport: Transport::new(Some(cookies))?,
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
        let address = Transport::address(&url(path)?, AccessMode::Direct)?;
        self.send(self.transport.client.get(address)).await
    }

    async fn post(&self, path: &str) -> Result<Value, String> {
        let address = Transport::address(&url(path)?, AccessMode::Direct)?;
        self.send(self.transport.client.post(address)).await
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
        let address = Transport::address(
            &url(&format!("/api/uploads/{upload_id}/blob"))?,
            AccessMode::Direct,
        )?;
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
        media_url(location)
    }

    pub async fn open_file(&self, upload_id: u64) -> Result<Url, String> {
        self.file_url(upload_id).await
    }

    /// Stream an attachment to `path` and return the number of bytes written.
    pub async fn save_file(&self, upload_id: u64, path: &Path) -> Result<u64, String> {
        let media = self.file_url(upload_id).await?;
        let client = reqwest::Client::builder()
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
        let mut file = tokio::fs::File::create(path)
            .await
            .map_err(|_| "无法写入所选位置")?;
        use tokio::io::AsyncWriteExt;
        let mut written = 0u64;
        while let Some(chunk) = response.chunk().await.map_err(|_| "附件下载中断")? {
            written += chunk.len() as u64;
            if written > MAX_DOWNLOAD {
                return Err("附件超过 2 GB，已停止下载".into());
            }
            file.write_all(&chunk).await.map_err(|_| "写入附件失败")?;
        }
        file.flush().await.map_err(|_| "写入附件失败")?;
        Ok(written)
    }
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
        "type": text(value.get("course_type")),
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

fn activity(value: &Value) -> Option<Value> {
    let id = value.get("id")?.as_i64()?;
    Some(json!({
        "id": id,
        "type": text(value.get("type")),
        "title": text(value.get("title")),
        "start": text(value.get("start_time")),
        "end": text(value.get("end_time")),
        "published": value.get("published").and_then(Value::as_bool).unwrap_or(true),
        "group": value.get("submit_by_group").and_then(Value::as_bool).unwrap_or(false),
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
        "start": text(value.get("start_time")),
        "end": text(value.get("end_time")),
        "published": value.get("published").and_then(Value::as_bool).unwrap_or(true),
        "description": plain_text(&text(data.get("description"))),
        "uploads": uploads,
        "submitByGroup": value.get("submit_by_group").and_then(Value::as_bool).unwrap_or(false),
        "submitCount": value.get("user_submit_count").and_then(Value::as_u64),
        "submitTimes": value.get("submit_times").and_then(Value::as_u64),
        "averageScore": number(value.get("average_score")),
        "highestScore": number(value.get("highest_score")),
        "lowestScore": number(value.get("lowest_score")),
        "scoredCount": value.get("has_score_count").and_then(Value::as_u64),
        "lessonStart": text(data.get("lesson_start")),
        "lessonEnd": text(data.get("lesson_end")),
        "duration": data.get("duration").and_then(Value::as_u64),
        "replayCode": replay_code(value),
        "liveName": text(external.and_then(|live| live.get("name"))),
        "liveStart": text(external.and_then(|live| live.get("start_time"))),
        "liveEnd": text(external.and_then(|live| live.get("end_time"))),
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
    fn media_and_player_urls_are_restricted() {
        assert!(media_url("https://media.xjtu.edu.cn/download/file/abc").is_ok());
        for bad in [
            "http://media.xjtu.edu.cn/download/file/abc",
            "https://media.xjtu.edu.cn.evil.test/file",
            "https://u:p@media.xjtu.edu.cn/file",
            "https://example.test/file",
        ] {
            assert!(media_url(bad).is_err(), "{bad}");
        }
        assert!(player_url(
            "https://rms-v5.xjtu.edu.cn/embed/lms/lesson-activity/a/detail?token=t"
        )
        .is_some());
        assert!(player_url("https://example.test/player").is_none());
        assert!(player_url("http://rms-v5.xjtu.edu.cn/player").is_none());
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
        match crate::auth::direct_login(LOGIN_URL, &account.username, &saved.0).await {
            Ok(cookies) => {
                let client = Client::with_cookies(&cookies).expect("client");
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
        let cookies = match crate::auth::direct_login(LOGIN_URL, &account.username, &saved.0).await
        {
            Ok(cookies) => cookies,
            Err(error) => {
                println!("login error: {error}");
                return;
            }
        };
        let client = Client::with_cookies(&cookies).expect("client");
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
    fn rejects_paths_outside_the_api() {
        assert!(url("/api/courses/1").is_ok());
        assert!(url("/user/index").is_err());
        assert!(url("/api/../admin").is_err());
    }
}
