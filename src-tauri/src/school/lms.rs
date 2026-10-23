//! 思源学堂 read-only adapter. The site keeps its own CAS cookie session, so
//! the account stores it under `sites.lms` and every call uses that jar.
use crate::{
    accounts::Account,
    network::{AccessMode, Transport},
};
use serde_json::{json, Value};
use url::Url;

const BASE: &str = "https://lms.xjtu.edu.cn";
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

fn url(path: &str) -> Result<Url, String> {
    if !path.starts_with("/api/") || path.contains("..") {
        return Err("思源学堂接口路径无效".into());
    }
    Url::parse(&format!("{BASE}{path}")).map_err(|_| "思源学堂接口地址无效".into())
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
            "type": "assignment",
            "title": "第一次作业",
            "end_time": "2026-10-30 23:59:00"
        }))
        .unwrap();
        assert_eq!(brief["type"], "assignment");
        assert_eq!(brief["end"], "2026-10-30 23:59:00");
        assert!(activity(&json!({"title": "missing id"})).is_none());
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

    #[test]
    fn rejects_paths_outside_the_api() {
        assert!(url("/api/courses/1").is_ok());
        assert!(url("/user/index").is_err());
        assert!(url("/api/../admin").is_err());
    }
}
