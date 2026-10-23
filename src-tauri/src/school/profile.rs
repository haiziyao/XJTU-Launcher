//! Student profile (hello) adapter. The service authenticates with a JWT that
//! the OAuth callback puts on its landing URL, not with the academic cookies.
use crate::{
    accounts::Account,
    network::{AccessMode, Transport},
};
use serde_json::{json, Value};
use url::Url;

const PROFILE_URL: &str = "https://hello.xjtu.edu.cn/yingxin/user/afterLogin?synAccessSource=pc";
const REFERER: &str = "http://hello.xjtu.edu.cn/yingxin-pc/";

fn text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn picture(value: &str) -> String {
    if value.starts_with("//") {
        format!("https:{value}")
    } else if let Some(path) = value.strip_prefix("http://") {
        format!("https://{path}")
    } else {
        value.to_string()
    }
}

fn body(payload: &Value) -> Result<Value, String> {
    if payload.get("state").and_then(Value::as_u64) != Some(200) {
        return Err("学籍登录已失效，请刷新学籍档案后重试".into());
    }
    let data = payload.get("data").filter(|data| data.is_object());
    let student = data
        .and_then(|data| data.get("studentBean"))
        .filter(|student| student.is_object());
    let teacher = student
        .and_then(|student| student.get("teacherBean"))
        .or_else(|| data.and_then(|data| data.get("teacherBean")))
        .filter(|teacher| teacher.is_object());
    let field = |name: &str| student.map(|s| text(s.get(name))).unwrap_or_default();
    let sno = field("sno");
    let student_no = if sno.is_empty() {
        field("account")
    } else {
        sno
    };
    let sex = match field("sex").as_str() {
        "1" => "男".to_string(),
        "2" => "女".to_string(),
        other => other.to_string(),
    };
    let major = field("professionName");
    // The server prefixes the major with its numeric code.
    let major = major
        .split_once(' ')
        .filter(|(code, _)| code.len() >= 4 && code.bytes().all(|b| b.is_ascii_digit()))
        .map(|(_, name)| name.trim().to_string())
        .unwrap_or(major);
    let picture_url = data
        .map(|data| text(data.get("studentPictureUrl")))
        .filter(|url| !url.is_empty())
        .unwrap_or_else(|| field("pictureUrl"));
    Ok(json!({
        "name": field("name"),
        "studentNo": student_no,
        "sex": sex,
        "birthdate": field("birthdate"),
        "grade": field("grade"),
        "campus": field("campusName"),
        "academy": field("academyName"),
        "department": field("departmentName"),
        "major": major,
        "className": field("className"),
        "schoolingLen": field("schoolingLen"),
        "enterSchoolDate": field("enterSchoolDate"),
        "cardId": field("cardId"),
        "pictureUrl": picture(&picture_url),
        "classTeacher": teacher.map(|t| text(t.get("classTeaName"))).unwrap_or_default(),
        "classTeacherPhone": teacher.map(|t| text(t.get("classTeaPhone"))).unwrap_or_default(),
        "counselor": teacher.map(|t| text(t.get("counselorName"))).unwrap_or_default(),
        "counselorPhone": teacher.map(|t| text(t.get("counselorPhone"))).unwrap_or_default(),
        "counselorOffice": teacher.map(|t| text(t.get("counselorOffLoca"))).unwrap_or_default(),
    }))
}

pub async fn query(account: &Account) -> Result<Value, String> {
    let site = account.sites.get("profile").ok_or("请先完成学籍档案登录")?;
    let token = site
        .token
        .as_deref()
        .filter(|token| !token.is_empty())
        .ok_or("请先完成学籍档案登录")?;
    let saved = site.cookies.as_deref().unwrap_or(&account.cookies);
    let transport = Transport::new(Some(saved))?;
    let url = Url::parse(PROFILE_URL).map_err(|_| "学籍接口地址无效")?;
    let response = transport
        .client
        .get(Transport::address(&url, AccessMode::Direct)?)
        .header("Referer", REFERER)
        .header("access-token", token)
        .header("access_token", token)
        .header(
            "systemtype",
            site.system_type.as_deref().unwrap_or("yingxin_student_pc"),
        )
        .header("synAccessSource", "pc")
        .send()
        .await
        .map_err(|_| "无法连接学籍服务，请检查网络后重试")?;
    if !response.status().is_success() {
        return Err("学籍登录已失效，请刷新学籍档案后重试".into());
    }
    let payload: Value = response
        .json()
        .await
        .map_err(|_| "学籍服务返回了非预期数据")?;
    body(&payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_the_student_and_teacher_beans() {
        let payload = json!({
            "state": 200,
            "data": {
                "studentPictureUrl": "//hello.xjtu.edu.cn/a.png",
                "studentBean": {
                    "name": "张三",
                    "sno": "2023000000",
                    "sex": "1",
                    "grade": "2023",
                    "campusName": "兴庆校区",
                    "academyName": "电信学部",
                    "professionName": "080901 计算机科学与技术",
                    "className": "计算机2301",
                    "teacherBean": {"classTeaName": "李老师", "counselorPhone": "123"}
                }
            }
        });
        let profile = body(&payload).unwrap();
        assert_eq!(profile["name"], "张三");
        assert_eq!(profile["sex"], "男");
        assert_eq!(profile["major"], "计算机科学与技术");
        assert_eq!(profile["pictureUrl"], "https://hello.xjtu.edu.cn/a.png");
        assert_eq!(profile["classTeacher"], "李老师");
        assert_eq!(profile["counselorPhone"], "123");
    }

    #[test]
    fn rejects_an_expired_token_and_tolerates_missing_beans() {
        assert!(body(&json!({"state": 401})).is_err());
        let profile = body(&json!({"state": 200})).unwrap();
        assert_eq!(profile["name"], "");
        assert_eq!(profile["studentNo"], "");
    }
}
