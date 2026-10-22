use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

pub fn domain(degree: &str) -> Result<&'static str, String> {
    match degree {
        "undergraduate" => Ok("bk-kq.xjtu.edu.cn"),
        "postgraduate" => Ok("yjs-kq.xjtu.edu.cn"),
        _ => Err("不支持的校园身份".into()),
    }
}

pub fn callback(url: &Url, degree: &str) -> Option<(String, String)> {
    if url.scheme() != "https"
        || url.host_str() != domain(degree).ok()
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    Some((
        query.get("loginRequestId")?.clone(),
        query.get("ticket")?.clone(),
    ))
    .filter(|(request, ticket)| !request.is_empty() && !ticket.is_empty())
}

async fn response_json(response: reqwest::Response) -> Result<Value, String> {
    if [401, 403].contains(&response.status().as_u16()) {
        return Err("校园会话已失效，请重新登录".into());
    }
    if !response.status().is_success() {
        return Err(format!(
            "校园系统暂不可用（HTTP {}）",
            response.status().as_u16()
        ));
    }
    // Do not forward response bodies/errors: they may contain tokens or personal data.
    let json: Value = response
        .json()
        .await
        .map_err(|_| "校园系统返回了非预期数据，请检查网络或重新登录")?;
    if json.get("code").and_then(Value::as_i64) != Some(0) {
        return Err("校园系统拒绝了请求，请重新登录后重试".into());
    }
    json.get("data")
        .cloned()
        .ok_or("校园系统响应缺少 data 字段".into())
}

async fn get(
    session: &crate::vault::Session,
    path: &str,
    query: &[(&str, String)],
) -> Result<Value, String> {
    let transport = crate::network::Transport::new(session.cookies.as_deref())?;
    let mut url = Url::parse(&format!("https://{}/sa{}", domain(&session.degree)?, path))
        .map_err(|_| "校园地址无效")?;
    url.query_pairs_mut()
        .extend_pairs(query.iter().map(|(k, v)| (*k, v.as_str())));
    let response = transport
        .client
        .get(crate::network::Transport::address(&url, session.mode)?)
        .header("X-Business-Token", &session.token)
        .send()
        .await
        .map_err(|_| "无法连接校园课表服务，请检查网络或重新登录")?;
    response_json(response).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Semester {
    semester_id: Value,
    academic_year: String,
    semester_name: String,
    start_date: String,
    end_date: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timetable {
    semester: String,
    start_date: String,
    end_date: String,
    courses: Vec<Course>,
}

#[derive(Deserialize, Serialize)]
pub struct Course {
    name: String,
    teacher: String,
    location: String,
    day: u8,
    start: u8,
    end: u8,
    weeks: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCourse {
    course_name: String,
    #[serde(default)]
    teacher_name: Option<String>,
    #[serde(default)]
    classroom_name: Option<String>,
    day_of_week: u8,
    start_section: u8,
    end_section: u8,
    week_ranges: String,
}

pub fn parse_weeks(text: &str) -> Result<Vec<u8>, String> {
    let mut weeks = vec![];
    let text = text.replace(['，', '、', ';', '；'], ",").replace('周', "");
    for part in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let odd = part.contains('单');
        let even = part.contains('双');
        let clean = part.replace(['(', ')', '（', '）', '单', '双', ' '], "");
        let values: Vec<_> = clean.split('-').collect();
        if values.len() > 2 {
            return Err("课表周次格式不受支持".into());
        }
        let start: u8 = values[0].parse().map_err(|_| "课表周次格式不受支持")?;
        let end: u8 = values
            .last()
            .unwrap()
            .parse()
            .map_err(|_| "课表周次格式不受支持")?;
        if start == 0 || start > end || end > 60 {
            return Err("课表周次超出范围".into());
        }
        weeks.extend((start..=end).filter(|w| (!odd || w % 2 == 1) && (!even || w % 2 == 0)));
    }
    weeks.sort();
    weeks.dedup();
    Ok(weeks)
}

fn normalize_courses(value: Value) -> Result<Vec<Course>, String> {
    let raw: Vec<RawCourse> =
        serde_json::from_value(value).map_err(|_| "课表字段发生变化，需要更新课表插件")?;
    raw.into_iter()
        .map(|c| {
            if !(1..=7).contains(&c.day_of_week)
                || c.start_section == 0
                || c.start_section > c.end_section
                || c.end_section > 16
            {
                return Err("课表节次超出支持范围".into());
            }
            Ok(Course {
                name: c.course_name,
                teacher: c.teacher_name.unwrap_or_default(),
                location: c.classroom_name.unwrap_or_default(),
                day: c.day_of_week,
                start: c.start_section,
                end: c.end_section,
                weeks: parse_weeks(&c.week_ranges)?,
            })
        })
        .collect()
}

/// Whether the campus host answers at all in the given access mode. Any HTTP
/// status counts: the sidebar self-check reports network reachability, not auth.
pub async fn reachable(
    session: &crate::vault::Session,
    mode: crate::network::AccessMode,
) -> bool {
    let Ok(host) = domain(&session.degree) else {
        return false;
    };
    let Ok(transport) = crate::network::Transport::with_timeout(
        session.cookies.as_deref(),
        std::time::Duration::from_secs(6),
    ) else {
        return false;
    };
    let Ok(url) = Url::parse(&format!(
        "https://{host}/sa/student/service/timetable/semesters"
    )) else {
        return false;
    };
    let Ok(target) = crate::network::Transport::address(&url, mode) else {
        return false;
    };
    transport
        .client
        .get(target)
        .header("X-Business-Token", &session.token)
        .send()
        .await
        .is_ok()
}

pub async fn timetable(session: &crate::vault::Session) -> Result<Timetable, String> {
    let semesters: Vec<Semester> =
        serde_json::from_value(get(session, "/student/service/timetable/semesters", &[]).await?)
            .map_err(|_| "学期字段发生变化，需要更新课表插件")?;
    let semester = semesters
        .into_iter()
        .next()
        .ok_or("校园系统没有返回学期信息")?;
    let id = match &semester.semester_id {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => return Err("学期 ID 格式不受支持".into()),
    };
    let weekly = get(
        session,
        "/student/service/timetable/weekly",
        &[("semesterId", id)],
    )
    .await?;
    let courses = normalize_courses(
        weekly
            .get("courses")
            .cloned()
            .ok_or("课表响应缺少 courses 字段")?,
    )?;
    Ok(Timetable {
        semester: format!("{} · {}", semester.academic_year, semester.semester_name),
        start_date: semester.start_date,
        end_date: semester.end_date,
        courses,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_requires_correct_school_host_and_ticket() {
        let valid = "https://bk-kq.xjtu.edu.cn/student-pc?loginRequestId=abc&ticket=ST-123";
        assert!(callback(&Url::parse(valid).unwrap(), "undergraduate").is_some());
        for url in [
            valid.replace("https", "http"),
            valid.replace("bk-kq.xjtu.edu.cn", "bk-kq.xjtu.edu.cn.evil.test"),
            valid.replace("bk-kq", "yjs-kq"),
            valid.replace("ST-123", ""),
        ] {
            assert!(callback(&Url::parse(&url).unwrap(), "undergraduate").is_none());
        }
    }

    #[test]
    fn week_ranges_include_alternating_weeks_and_deduplicate() {
        assert_eq!(
            parse_weeks("1-5(单), 5，8-10双周").unwrap(),
            vec![1, 3, 5, 8, 10]
        );
        assert!(parse_weeks("60-2").is_err());
        assert!(parse_weeks("1-255").is_err());
    }

    #[test]
    fn normalizes_response_and_rejects_invalid_days() {
        let mut value = serde_json::json!([{"courseName":"高等数学", "teacherName":null, "classroomName":"主楼101", "dayOfWeek":1, "startSection":1, "endSection":2, "weekRanges":"1-16"}]);
        let courses = normalize_courses(value.clone()).unwrap();
        assert_eq!(courses[0].weeks.len(), 16);
        assert_eq!(courses[0].teacher, "");
        value[0]["dayOfWeek"] = 8.into();
        assert!(normalize_courses(value).is_err());
    }
}
