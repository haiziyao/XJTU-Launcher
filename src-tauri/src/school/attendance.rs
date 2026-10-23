//! Attendance streams and course records. Protocol adapted from the reference
//! toolbox's attendance module; the account already holds this service's token.
use crate::{
    accounts::Account,
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

    #[test]
    fn accepts_numeric_and_string_totals() {
        assert_eq!(total(&json!({"total": 12})), 12);
        assert_eq!(total(&json!({"total": "34"})), 34);
        assert_eq!(total(&json!({})), 0);
        assert!(rows(&json!({"rows": []})).is_ok());
        assert!(rows(&json!({})).is_err());
    }
}
