//! Attendance streams and course records. Protocol adapted from the reference
//! toolbox's attendance module; the account already holds this service's token.
use crate::{
    accounts::Account,
    campus::{Course, Timetable},
    network::{AccessMode, Transport},
};
use serde_json::{json, Map, Value};
use std::time::Duration;
use url::Url;

const PAGE_SIZE: u32 = 50;

pub struct Client {
    transport: Transport,
    token: String,
    mode: AccessMode,
    base: String,
}

impl Client {
    pub fn connect(account: Account) -> Result<Self, String> {
        let host = match account.degree.as_str() {
            "undergraduate" => "bk-kq.xjtu.edu.cn",
            "postgraduate" => "yjs-kq.xjtu.edu.cn",
            _ => return Err("不支持的校园身份".into()),
        };
        Ok(Self {
            transport: Transport::with_timeout(Some(&account.cookies), Duration::from_secs(15))?,
            token: account.token,
            mode: account.mode,
            base: format!("https://{host}/sa"),
        })
    }

    async fn post(&self, path: &str, body: &Value) -> Result<Value, String> {
        let logical =
            Url::parse(&format!("{}{path}", self.base)).map_err(|_| "考勤接口地址无效")?;
        let target = Transport::address(&logical, self.mode)?;
        let response = self
            .transport
            .client
            .post(target)
            .header("X-Business-Token", &self.token)
            .json(body)
            .send()
            .await
            .map_err(|_| "无法连接考勤服务，请检查网络或重新登录")?;
        let status = response.status();
        if status.is_redirection() || matches!(status.as_u16(), 401 | 403) {
            return Err("校园会话已失效，请重新登录校园账号".into());
        }
        if !status.is_success() {
            return Err(format!("考勤服务返回 HTTP {}", status.as_u16()));
        }
        let value: Value = response
            .json()
            .await
            .map_err(|_| "考勤服务返回了非预期数据")?;
        if value.get("code").and_then(Value::as_i64) != Some(0) {
            return Err("考勤系统拒绝了请求，请重新登录后重试".into());
        }
        Ok(value.get("data").cloned().unwrap_or(Value::Null))
    }

    /// Swipe streams ("有效/无效") for an optional date range.
    pub async fn streams(
        &self,
        start: Option<&str>,
        end: Option<&str>,
        page: u32,
    ) -> Result<Value, String> {
        let mut filter = Map::new();
        if let Some(start) = start {
            filter.insert("startDate".into(), json!(start));
        }
        if let Some(end) = end {
            filter.insert("endDate".into(), json!(end));
        }
        let data = self
            .post(
                "/student/pc/attendance-streams/page",
                &json!({"pageNum": page, "pageSize": PAGE_SIZE, "data": filter}),
            )
            .await?;
        let rows = rows(&data)?;
        Ok(json!({
            "page": page,
            "total": total(&data),
            "rows": rows.iter().map(|row| json!({
                "id": row.get("id").cloned().unwrap_or(Value::Null),
                "place": row.get("classroomName").cloned().unwrap_or(Value::Null),
                "time": row.get("collectTime").cloned().unwrap_or(Value::Null),
                "valid": row.get("effective").and_then(Value::as_bool).unwrap_or(false),
            })).collect::<Vec<_>>()
        }))
    }

    /// Per-course attendance results for a required date range.
    pub async fn records(&self, start: &str, end: &str, page: u32) -> Result<Value, String> {
        let data = self
            .post(
                "/student/pc/attendance-records/page",
                &json!({
                    "pageNum": page,
                    "pageSize": PAGE_SIZE,
                    "data": {"startDate": start, "endDate": end}
                }),
            )
            .await?;
        let rows = rows(&data)?;
        Ok(json!({
            "page": page,
            "total": total(&data),
            "rows": rows.iter().map(|row| json!({
                "id": row.get("resultId").cloned().unwrap_or(Value::Null),
                "date": row.get("attendanceDate").cloned().unwrap_or(Value::Null),
                "start": row.get("startSection").cloned().unwrap_or(Value::Null),
                "end": row.get("endSection").cloned().unwrap_or(Value::Null),
                "week": row.get("courseWeek").cloned().unwrap_or(Value::Null),
                "place": row.get("classroomName").cloned().unwrap_or(Value::Null),
                "teacher": row.get("teacherName").cloned().unwrap_or(Value::Null),
                "status": status(row.get("attendanceStatus").and_then(Value::as_str)),
            })).collect::<Vec<_>>()
        }))
    }
}

/// Fills `course` on every record by matching the cached timetable.
///
/// The attendance service returns only date, sections and room. The course is
/// recovered from the timetable: same weekday, overlapping section range, and
/// the date's week listed for that course. When two courses share a slot the
/// room decides, then the teacher; without a unique match the name stays empty
/// instead of guessing.
pub(crate) fn label_courses(data: &mut Value, timetable: &Timetable) {
    let Some(rows) = data.get_mut("rows").and_then(Value::as_array_mut) else {
        return;
    };
    let term_start = day_number(&timetable.start_date);
    for row in rows {
        let name = course_name(row, timetable, term_start).unwrap_or_default();
        if let Some(object) = row.as_object_mut() {
            object.insert("course".into(), Value::String(name));
        }
    }
}

fn course_name(row: &Value, timetable: &Timetable, term_start: Option<i64>) -> Option<String> {
    let date = text(row.get("date"));
    let date = date.get(..10)?;
    let weekday = weekday(date)?;
    let start = section(row.get("start"))?;
    let end = section(row.get("end")).unwrap_or(start);
    let week = week(row.get("week"), date, term_start)?;
    let place = normalized(&text(row.get("place")));
    let teacher = normalized(&text(row.get("teacher")));
    let mut candidates: Vec<&Course> = timetable
        .courses
        .iter()
        .filter(|course| {
            course.day == weekday
                && course.weeks.contains(&week)
                && course.start <= end
                && start <= course.end
        })
        .collect();
    if candidates.len() > 1 && !place.is_empty() {
        let by_place: Vec<&Course> = candidates
            .iter()
            .copied()
            .filter(|course| overlaps(&normalized(&course.location), &place))
            .collect();
        if !by_place.is_empty() {
            candidates = by_place;
        }
    }
    if candidates.len() > 1 && !teacher.is_empty() {
        let by_teacher: Vec<&Course> = candidates
            .iter()
            .copied()
            .filter(|course| overlaps(&normalized(&course.teacher), &teacher))
            .collect();
        if !by_teacher.is_empty() {
            candidates = by_teacher;
        }
    }
    (candidates.len() == 1).then(|| candidates[0].name.clone())
}

/// The record's own week when the service sends one, otherwise the week count
/// from the timetable's start date.
fn week(value: Option<&Value>, date: &str, term_start: Option<i64>) -> Option<u8> {
    if let Some(value) = digits(&text(value)) {
        return Some(value);
    }
    let days = day_number(date)? - term_start?;
    if days < 0 {
        return None;
    }
    u8::try_from(days / 7 + 1).ok().filter(|week| *week > 0)
}

/// Section numbers arrive as numbers or as strings such as `"第3节"`.
fn section(value: Option<&Value>) -> Option<u8> {
    let raw = match value? {
        Value::Number(number) => number.as_u64().map(|value| value.to_string()),
        other => Some(text(Some(other))),
    }?;
    digits(&raw)
}

fn digits(value: &str) -> Option<u8> {
    let run: String = value
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    run.parse().ok()
}

fn text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn normalized(value: &str) -> String {
    value.chars().filter(|c| !c.is_whitespace()).collect()
}

fn overlaps(left: &str, right: &str) -> bool {
    !left.is_empty() && !right.is_empty() && (left.contains(right) || right.contains(left))
}

/// Days since 1970-01-01 for a `YYYY-MM-DD` string, with holiday-free week
/// counting; the timetable uses the school's own start date.
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

/// Monday = 1 … Sunday = 7, matching the timetable.
fn weekday(day: &str) -> Option<u8> {
    let days = day_number(day)?;
    u8::try_from((days + 3).rem_euclid(7) + 1).ok()
}

fn rows(data: &Value) -> Result<&Vec<Value>, String> {
    data.get("rows")
        .and_then(Value::as_array)
        .ok_or_else(|| "考勤数据格式已变化".into())
}

fn total(data: &Value) -> u64 {
    data.get("total")
        .and_then(Value::as_u64)
        .or_else(|| {
            data.get("total")
                .and_then(Value::as_str)
                .and_then(|value| value.parse().ok())
        })
        .unwrap_or(0)
}

fn status(value: Option<&str>) -> &'static str {
    match value {
        Some("PENDING") => "pending",
        Some("NORMAL") => "normal",
        Some("LATE") => "late",
        Some("ABSENT") => "absent",
        Some("LEAVE") => "leave",
        Some("NOT_REQUIRED") => "notRequired",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_attendance_statuses_and_keeps_unknown() {
        assert_eq!(status(Some("NORMAL")), "normal");
        assert_eq!(status(Some("NOT_REQUIRED")), "notRequired");
        assert_eq!(status(None), "unknown");
        assert_eq!(status(Some("SOMETHING_NEW")), "unknown");
    }

    fn course(
        name: &str,
        day: u8,
        start: u8,
        end: u8,
        weeks: Vec<u8>,
        location: &str,
        teacher: &str,
    ) -> Course {
        Course {
            id: String::new(),
            name: name.into(),
            teacher: teacher.into(),
            location: location.into(),
            day,
            start,
            end,
            weeks,
            color: None,
            custom: false,
            edited: false,
        }
    }

    fn timetable(courses: Vec<Course>) -> Timetable {
        Timetable {
            semester: "2026-2027 · 秋季学期".into(),
            start_date: "2026-09-07".into(),
            end_date: "2027-01-17".into(),
            courses,
            hidden: Vec::new(),
        }
    }

    #[test]
    fn labels_records_from_the_timetable() {
        let timetable = timetable(vec![course(
            "高等数学",
            1,
            1,
            2,
            (1..=16).collect(),
            "主楼A101",
            "张三",
        )]);
        let mut data = json!({"rows":[
            {"date":"2026-09-14","start":1,"end":2,"week":"2"},
            {"date":"2026-09-15","start":1,"end":2,"week":"2"}
        ]});
        label_courses(&mut data, &timetable);
        assert_eq!(data["rows"][0]["course"], "高等数学");
        // Wrong weekday: no course, rather than the nearest one.
        assert_eq!(data["rows"][1]["course"], "");
    }

    #[test]
    fn derives_week_and_sections_when_the_record_omits_them() {
        let timetable = timetable(vec![course("大学物理", 2, 3, 4, vec![1, 2, 3], "", "")]);
        let mut data = json!({"rows":[{"date":"2026-09-22","start":"第3节","end":4}]});
        label_courses(&mut data, &timetable);
        assert_eq!(data["rows"][0]["course"], "大学物理");
        let mut out_of_weeks = json!({"rows":[{"date":"2026-11-03","start":3,"end":4}]});
        label_courses(&mut out_of_weeks, &timetable);
        assert_eq!(out_of_weeks["rows"][0]["course"], "");
    }

    #[test]
    fn room_breaks_a_shared_slot_and_ambiguity_stays_empty() {
        let timetable = timetable(vec![
            course("线性代数", 3, 1, 2, vec![5], "主楼B202", "李四"),
            course("概率论", 3, 1, 2, vec![5], "主楼C303", "王五"),
        ]);
        let mut data = json!({"rows":[
            {"date":"2026-10-07","start":1,"end":2,"week":"5","place":"B202"},
            {"date":"2026-10-07","start":1,"end":2,"week":"5","place":""}
        ]});
        label_courses(&mut data, &timetable);
        assert_eq!(data["rows"][0]["course"], "线性代数");
        assert_eq!(data["rows"][1]["course"], "");
    }

    #[test]
    fn parses_chinese_week_labels() {
        assert_eq!(digits("第3周"), Some(3));
        assert_eq!(digits("12"), Some(12));
        assert_eq!(digits(""), None);
    }

    #[test]
    fn accepts_numeric_and_string_totals() {
        assert_eq!(total(&json!({"total": 12})), 12);
        assert_eq!(total(&json!({"total": "34"})), 34);
        assert_eq!(total(&json!({})), 0);
        assert!(rows(&json!({"rows": []})).is_ok());
        assert!(rows(&json!({})).is_err());
    }
}
