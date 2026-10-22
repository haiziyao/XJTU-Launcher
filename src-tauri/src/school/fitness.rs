//! Fitness test scores from the physical education platform.
use crate::{accounts::Account, network::Transport};
use reqwest::{redirect::Policy, Client as Http};
use serde_json::{json, Value};
use std::time::Duration;
use url::Url;

const LOGIN: &str =
    "https://tyxylp.xjtu.edu.cn/bdlp_h5_fitness_test/public/index.php/index/login/xjtuLogin";
const YEARS: &str =
    "https://tyxylp.xjtu.edu.cn/bdlp_h5_fitness_test/public/index.php/index/fitness/fitnessYear";
const SCORE: &str =
    "https://tyxylp.xjtu.edu.cn/bdlp_h5_fitness_test/public/index.php/index/Report/getStudentScore";
const H5_HOME: &str =
    "https://tyxylp.xjtu.edu.cn/bdlp_h5_fitness_test/view/h5xajt/#/pages/index/index";

const ITEMS: [(&str, &str); 7] = [
    ("bmi", "身高体重"),
    ("vc", "肺活量"),
    ("jump", "立定跳远"),
    ("sit_and_reach", "坐位体前屈"),
    ("pull_and_sit", "引体向上 / 仰卧起坐"),
    ("50m", "50 米"),
    ("run", "800 / 1000 米"),
];

fn campus_host(url: &Url) -> bool {
    url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|host| host == "xjtu.edu.cn" || host.ends_with(".xjtu.edu.cn"))
}

pub struct Client {
    http: Http,
}

impl Client {
    pub async fn connect(account: &Account) -> Result<Self, String> {
        let transport = Transport::new(Some(&account.cookies))?;
        let http = Http::builder()
            .cookie_provider(transport.cookies.clone())
            .redirect(Policy::custom(|attempt| {
                if campus_host(attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|_| "无法创建体测连接")?;
        let response = http
            .get(LOGIN)
            .send()
            .await
            .map_err(|_| "无法连接体测服务，请检查网络后重试")?;
        if !response.status().is_success() {
            return Err("体测登录态已失效，请重新登录校园账号".into());
        }
        let client = Self { http };
        // A real query proves the session, not just the login redirect.
        client.years().await?;
        Ok(client)
    }

    async fn post(&self, url: &str, form: &[(&str, &str)]) -> Result<Value, String> {
        let response = self
            .http
            .post(url)
            .header("Origin", "https://tyxylp.xjtu.edu.cn")
            .header("Referer", H5_HOME)
            .header("X-Requested-With", "XMLHttpRequest")
            .form(form)
            .send()
            .await
            .map_err(|_| "体测请求未收到响应")?;
        let status = response.status();
        if status.is_redirection() || matches!(status.as_u16(), 401 | 403) {
            return Err("体测登录态已失效，请重新登录校园账号".into());
        }
        if !status.is_success() {
            return Err(format!("体测服务返回 HTTP {}", status.as_u16()));
        }
        let bytes = crate::network::limited_bytes(response, 2 * 1024 * 1024).await?;
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| "体测返回了非预期数据")?;
        if value.get("status").and_then(Value::as_i64) != Some(1) {
            return Err(text(value.get("info")).unwrap_or_else(|| "体测请求失败".into()));
        }
        Ok(value.get("data").cloned().unwrap_or(Value::Null))
    }

    pub async fn years(&self) -> Result<Vec<Value>, String> {
        let data = self.post(YEARS, &[("from", "1")]).await?;
        let list = data
            .get("list")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(list
            .iter()
            .filter_map(|item| {
                let year = item.get("year_num").and_then(|value| match value {
                    Value::String(text) => Some(text.clone()),
                    Value::Number(number) => Some(number.to_string()),
                    _ => None,
                })?;
                Some(json!({
                    "year": year,
                    "name": text(item.get("name")).unwrap_or_else(|| year.clone()),
                    "checked": item.get("checked").and_then(Value::as_bool).unwrap_or(false),
                }))
            })
            .collect())
    }

    pub async fn score(&self, year: &str) -> Result<Value, String> {
        let data = self.post(SCORE, &[("year_num", year)]).await?;
        let items = ITEMS
            .iter()
            .map(|(key, name)| {
                let score_key =
                    if *key == "bmi" && data.get("bmi_score_new").is_some_and(|v| !v.is_null()) {
                        "bmi_score_new"
                    } else {
                        ""
                    };
                let score = if score_key.is_empty() {
                    text(data.get(format!("{key}_score")))
                } else {
                    text(data.get(score_key))
                };
                json!({
                    "key": key,
                    "name": name,
                    "score": score.unwrap_or_default(),
                    "grade": text(data.get(format!("{key}_grade"))).unwrap_or_default(),
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({
            "studentNumber": text(data.get("student_num")).unwrap_or_default(),
            "studentName": text(data.get("student_name")).unwrap_or_default(),
            "totalScore": text(data.get("total_score")).unwrap_or_default(),
            "totalGrade": text(data.get("total_grade")).unwrap_or_default(),
            "reportType": text(data.get("report_type")).unwrap_or_default(),
            "reportStatus": text(data.get("report_status")).unwrap_or_default(),
            "sex": text(data.get("sex")).unwrap_or_default(),
            "grade": text(data.get("grade")).unwrap_or_default(),
            "items": items,
        }))
    }
}

fn text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.trim().to_string()).filter(|text| !text.is_empty()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}
