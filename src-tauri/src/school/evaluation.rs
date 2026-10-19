use super::transport::{condition, field, required, success, Client};
use crate::accounts::Account;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use uuid::Uuid;

const ROOT: &str = "/jwapp/sys/wspjyyapp";
const TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Questionnaire {
    pub id: String,
    pub course: String,
    pub teacher: String,
    pub kind: String,
    pub term: String,
    pub deadline: String,
    pub finished: bool,
    #[serde(skip)]
    raw: Value,
}
impl Questionnaire {
    fn from_row(raw: Value, finished: bool) -> Result<Self, String> {
        let identity = ["WJDM", "PCDM", "BPR", "JXBID", "PGNR", "PGLXDM"]
            .iter()
            .map(|k| required(&raw, k))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            id: serde_json::to_string(&identity).map_err(|_| "评教标识无效")?,
            course: required(&raw, "KCM")?,
            teacher: field(&raw, "BPJS"),
            kind: field(&raw, "PGLXDM"),
            term: field(&raw, "XNXQDM"),
            deadline: field(&raw, "JSSJ"),
            finished,
            raw,
        })
    }
}
pub(super) async fn list(
    client: &Client,
    term: Option<String>,
    finished: bool,
) -> Result<Vec<Questionnaire>, String> {
    let term = match term {
        Some(t) => t,
        None => {
            let setting = json!([condition("CSDM","PJGLPJSJ"),{"name":"ZCSDM","value":"PJXNXQ","builder":"m_value_equal","linkOpt":"AND"}]);
            let data = client
                .rows(
                    &format!("{ROOT}/modules/xspj/cxxtcs.do"),
                    "cxxtcs",
                    &[("setting".into(), setting.to_string())],
                )
                .await?;
            required(data.first().ok_or("当前没有开放的评教学期")?, "CSZA")?
        }
    };
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for kind in ["01", "05"] {
        let records = client
            .paged(
                &format!("{ROOT}/modules/xspj/cxdwpj.do"),
                "cxdwpj",
                &[
                    ("PGLXDM".into(), kind.into()),
                    ("SFPG".into(), if finished { "1" } else { "0" }.into()),
                    ("SFKF".into(), "1".into()),
                    ("SFFB".into(), "1".into()),
                    ("XNXQDM".into(), term.clone()),
                ],
            )
            .await?;
        for row in records {
            let q = Questionnaire::from_row(row, finished)?;
            if seen.insert(q.id.clone()) {
                result.push(q);
            }
        }
    }
    Ok(result)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Prepare {
    pub ids: Vec<String>,
    /// 1 is the first/highest option, 5 the fifth/lowest; actual answers are shown in preview.
    pub rank: u8,
    pub comment: String,
    #[serde(default)]
    pub overrides: BTreeMap<String, BTreeMap<String, String>>,
}
impl Prepare {
    fn validate(&self) -> Result<(), String> {
        if self.ids.is_empty()
            || self.ids.len() > 32
            || !(1..=5).contains(&self.rank)
            || self.comment.chars().count() > 4000
            || self.ids.iter().any(|id| id.len() > 4096)
            || self.ids.iter().collect::<HashSet<_>>().len() != self.ids.len()
        {
            return Err("请选择 1–32 份不同问卷和有效评分模板，文字意见不超过 4000 字".into());
        }
        if self.overrides.len() > 32
            || self.overrides.values().any(|q| {
                q.len() > 200
                    || q.iter()
                        .any(|(k, v)| k.len() > 256 || v.chars().count() > 4000)
            })
        {
            return Err("自定义答案数量或长度超出限制".into());
        }
        Ok(())
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewAnswer {
    pub id: String,
    pub question: String,
    pub kind: String,
    pub answer: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewCourse {
    pub questionnaire: Questionnaire,
    pub answers: Vec<PreviewAnswer>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub id: String,
    pub account_id: String,
    pub expires_in_seconds: u64,
    pub courses: Vec<PreviewCourse>,
}
struct PreparedCourse {
    preview: PreviewCourse,
    payload: Value,
}
struct Draft {
    created: Instant,
    account_id: String,
    generation: u64,
    courses: Vec<PreparedCourse>,
}
#[derive(Default)]
pub struct Manager {
    drafts: Mutex<HashMap<String, Draft>>,
    running: Mutex<HashMap<String, Arc<AtomicBool>>>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub questionnaire_id: String,
    pub course: String,
    pub status: &'static str,
    pub message: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub id: String,
    pub account_id: String,
    pub completed: usize,
    pub total: usize,
    pub outcome: Outcome,
}
impl Manager {
    pub async fn prepare(
        &self,
        account: Account,
        generation: u64,
        input: Prepare,
    ) -> Result<(Preview, String), String> {
        input.validate()?;
        let client = Client::connect(account).await?;
        let available = list(&client, None, false).await?;
        let mut courses = Vec::new();
        for id in &input.ids {
            let questionnaire = available
                .iter()
                .find(|q| &q.id == id)
                .ok_or("所选问卷已完成或不再开放，请刷新后重新选择")?
                .clone();
            let q = &questionnaire.raw;
            let student = client
                .account
                .student_id
                .as_deref()
                .ok_or("当前账号缺少已验证学号，请重新登录获取基础资料")?;
            let data = client
                .rows(
                    &format!("{ROOT}/modules/wj/cxwjzb.do"),
                    "cxwjzb",
                    &[
                        ("WJDM".into(), required(q, "WJDM")?),
                        ("JXBID".into(), required(q, "JXBID")?),
                    ],
                )
                .await?;
            let mut fields = vec![("CPR".into(), student.into()), ("SFPG".into(), "0".into())];
            let mut filters = vec![condition("CPR", student)];
            for key in ["WJDM", "PCDM", "BPR", "PGNR", "JXBID"] {
                let value = required(q, key)?;
                fields.push((key.into(), value.clone()));
                filters.push(condition(key, &value));
            }
            fields.push(("querySetting".into(), json!(filters).to_string()));
            let options = client
                .rows(
                    &format!("{ROOT}/modules/wj/cxxswjzbxq.do"),
                    "cxxswjzbxq",
                    &fields,
                )
                .await?;
            let (answers, payload) = build_answers(q, student, &data, &options, &input)?;
            courses.push(PreparedCourse {
                preview: PreviewCourse {
                    questionnaire,
                    answers,
                },
                payload,
            });
        }
        let id = Uuid::new_v4().to_string();
        let preview = Preview {
            id: id.clone(),
            account_id: client.account.id.clone(),
            expires_in_seconds: TTL.as_secs(),
            courses: courses.iter().map(|c| c.preview.clone()).collect(),
        };
        let mut drafts = self.drafts.lock().map_err(|_| "评教预览正在使用")?;
        drafts.retain(|_, d| d.created.elapsed() < TTL);
        if drafts.len() >= 8 {
            return Err("评教预览过多，请稍后重新生成".into());
        }
        drafts.insert(
            id,
            Draft {
                created: Instant::now(),
                account_id: client.account.id.clone(),
                generation,
                courses,
            },
        );
        Ok((preview, client.transport.snapshot()?))
    }
    pub fn cancel(&self, id: &str) -> Result<(), String> {
        if let Some(flag) = self.running.lock().map_err(|_| "评教任务正在使用")?.get(id) {
            flag.store(true, Ordering::SeqCst);
        }
        self.drafts
            .lock()
            .map_err(|_| "评教预览正在使用")?
            .remove(id);
        Ok(())
    }
    /// Only called by the explicit submission command. Never auto-retries a POST.
    pub async fn submit(
        &self,
        id: &str,
        account: Account,
        generation: u64,
        on_progress: impl Fn(Progress),
    ) -> Result<(Vec<Outcome>, Option<String>), String> {
        let draft = self
            .drafts
            .lock()
            .map_err(|_| "评教预览正在使用")?
            .remove(id)
            .ok_or("评教预览不存在、已提交或已取消，请重新预览")?;
        validate_draft(&draft, &account.id, generation)?;
        let flag = Arc::new(AtomicBool::new(false));
        self.running
            .lock()
            .map_err(|_| "评教任务正在使用")?
            .insert(id.into(), flag.clone());
        let result = async {
            let client = Client::connect(account).await?;
            let still_open = list(&client, None, false).await?;
            let mut outcomes = Vec::new();
            let total = draft.courses.len();
            let mut uncertain = false;
            for course in draft.courses {
                let q = &course.preview.questionnaire;
                let (status, message) = if flag.load(Ordering::SeqCst) {
                    ("cancelled", "已取消，未发送此问卷".into())
                } else if uncertain {
                    ("skipped", "前一问卷结果待确认，未继续发送".into())
                } else if !still_open.iter().any(|open| open.id == q.id) {
                    ("skipped", "问卷已完成或不再开放，请刷新状态".into())
                } else {
                    let response = client
                        .post(
                            &format!("{ROOT}/WspjwjController/addXsPgysjg.do"),
                            &[("requestParamStr".into(), course.payload.to_string())],
                        )
                        .await;
                    match response {
                        Ok(v) if success(&v["datas"]["code"]) => {
                            ("submitted", "学校已确认提交成功".into())
                        }
                        Ok(_) => ("failed", "学校未接受此问卷，请核对答案与开放时间".into()),
                        Err(_) => {
                            uncertain = true;
                            (
                                "unknown",
                                "未能确认学校是否收到，请刷新问卷状态；不会自动重试".into(),
                            )
                        }
                    }
                };
                let outcome = Outcome {
                    questionnaire_id: q.id.clone(),
                    course: q.course.clone(),
                    status,
                    message,
                };
                outcomes.push(outcome.clone());
                on_progress(Progress {
                    id: id.into(),
                    account_id: client.account.id.clone(),
                    completed: outcomes.len(),
                    total,
                    outcome,
                });
            }
            Ok((outcomes, client.transport.snapshot().ok()))
        }
        .await;
        self.running
            .lock()
            .map_err(|_| "评教任务正在使用")?
            .remove(id);
        result
    }
}
fn validate_draft(draft: &Draft, account_id: &str, generation: u64) -> Result<(), String> {
    if draft.created.elapsed() >= TTL {
        return Err("评教预览已过期，请重新预览".into());
    }
    if draft.account_id != account_id || draft.generation != generation {
        return Err("账号或会话已变化，请重新生成评教预览".into());
    }
    Ok(())
}
fn build_answers(
    q: &Value,
    student: &str,
    questions: &[Value],
    options: &[Value],
    input: &Prepare,
) -> Result<(Vec<PreviewAnswer>, Value), String> {
    if questions.is_empty() || questions.len() > 200 {
        return Err("问卷题目数量无效".into());
    }
    let id = Questionnaire::from_row(q.clone(), false)?.id;
    let overrides = input.overrides.get(&id);
    let mut ids = HashSet::new();
    let mut payload = Vec::new();
    let mut preview = Vec::new();
    for question in questions {
        let key = required(question, "ZBDM")?;
        if !ids.insert(key.clone()) {
            return Err("问卷存在重复题号".into());
        }
        let title = required(question, "ZBMC")?;
        let kind = required(question, "TXDM")?;
        let custom = overrides.and_then(|m| m.get(&key));
        let required_answer = field(question, "SFBT") == "1";
        let (answer, subjective, display) = match kind.as_str() {
            "01" => {
                let choices: Vec<_> = options.iter().filter(|o| field(o, "ZBDM") == key).collect();
                let selected = if let Some(value) = custom {
                    choices
                        .iter()
                        .find(|o| field(o, "DAFXDM") == *value)
                        .copied()
                } else {
                    choices
                        .iter()
                        .filter_map(|o| {
                            field(o, "DAPX")
                                .parse::<i32>()
                                .ok()
                                .map(|rank| ((rank - i32::from(input.rank)).abs(), rank, *o))
                        })
                        .min_by_key(|(distance, rank, _)| (*distance, *rank))
                        .map(|(_, _, o)| o)
                }
                .ok_or("题目没有与答案匹配的可选项，请重新预览")?;
                let answer = required(selected, "DAFXDM")?;
                let label = ["DAFXMC", "DAXX", "DAMC", "MC"]
                    .iter()
                    .map(|k| field(selected, k))
                    .find(|v| !v.is_empty())
                    .unwrap_or_else(|| {
                        format!(
                            "选项 {}（分值 {}）",
                            field(selected, "DAPX"),
                            field(selected, "FZ")
                        )
                    });
                (answer, String::new(), label)
            }
            "02" => {
                let value = custom.unwrap_or(&input.comment).trim().to_string();
                if required_answer && value.is_empty() {
                    return Err("问卷含必填文字题，请填写文字意见或逐题答案".into());
                }
                (String::new(), value.clone(), value)
            }
            "03" => {
                let max = required(question, "FZ")?
                    .parse::<f64>()
                    .map_err(|_| "问卷分值上限无效")?;
                let score = if let Some(value) = custom {
                    value.parse::<f64>().map_err(|_| "分值答案应为数字")?
                } else {
                    (max * (6.0 - f64::from(input.rank)) / 5.0).round()
                };
                if !max.is_finite() || max < 0.0 || !score.is_finite() || score < 0.0 || score > max
                {
                    return Err("分值答案超出题目范围".into());
                }
                (score.to_string(), String::new(), format!("{score} / {max}"))
            }
            _ => return Err("问卷包含尚未适配的题型，请通过学校网页填写".into()),
        };
        let mut row = json!({"CPR":student,"DA":answer,"ZGDA":subjective,"DAXH":"1","SFXYTJFJXX":"","FJXXSFBT":"","FJXX":""});
        for k in ["WJDM", "BPR", "PGNR", "PCDM", "JXBID"] {
            row[k] = json!(required(q, k)?);
        }
        for k in ["ZBDM", "TXDM", "ZBMC", "DADM", "SFBT", "FZ"] {
            row[k] = question.get(k).cloned().unwrap_or(Value::Null);
        }
        if field(question, "DADM").is_empty() {
            return Err("问卷缺少答案代码，不能生成提交数据".into());
        }
        preview.push(PreviewAnswer {
            id: key,
            question: title,
            kind,
            answer: display,
        });
        payload.push(row);
    }
    if overrides.is_some_and(|m| m.keys().any(|k| !ids.contains(k))) {
        return Err("自定义答案包含已不存在的题号".into());
    }
    let payload = json!({"WJDM":required(q,"WJDM")?,"PCDM":required(q,"PCDM")?,"PGLY":"1","SFTJ":"1","WJYSJG":serde_json::to_string(&payload).map_err(|_|"无法编码问卷答案")?});
    Ok((preview, payload))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn questionnaire() -> Value {
        json!({"WJDM":"q","PCDM":"term","BPR":"teacher","JXBID":"class","PGNR":"course","PGLXDM":"01","KCM":"Test"})
    }
    fn input() -> Prepare {
        Prepare {
            ids: vec!["q".into()],
            rank: 2,
            comment: "My own review".into(),
            overrides: BTreeMap::new(),
        }
    }
    #[test]
    fn preview_is_the_exact_submission_and_preserves_school_ids() {
        let questions = vec![
            json!({"ZBDM":"one","ZBMC":"Choice","TXDM":"01","DADM":"answer-code","SFBT":"1"}),
            json!({"ZBDM":"two","ZBMC":"Comment","TXDM":"02","DADM":"text-code","SFBT":"1"}),
        ];
        let options = vec![
            json!({"ZBDM":"one","DAFXDM":"best","DAPX":"1","DAFXMC":"Excellent"}),
            json!({"ZBDM":"one","DAFXDM":"good","DAPX":"2","DAFXMC":"Good"}),
        ];
        let (view, payload) =
            build_answers(&questionnaire(), "student", &questions, &options, &input()).unwrap();
        assert_eq!(view[0].answer, "Good");
        assert_eq!(view[1].answer, "My own review");
        let submitted: Vec<Value> =
            serde_json::from_str(payload["WJYSJG"].as_str().unwrap()).unwrap();
        assert_eq!(submitted[0]["DA"], "good");
        assert_eq!(submitted[0]["CPR"], "student");
        assert_eq!(submitted[0]["BPR"], "teacher");
        assert_eq!(submitted[1]["ZGDA"], view[1].answer);
        let mut missing = input();
        missing.comment.clear();
        assert!(
            build_answers(&questionnaire(), "student", &questions, &options, &missing).is_err()
        );
    }
    #[test]
    fn rejects_unknown_question_types_and_invalid_selections() {
        let q = vec![json!({"ZBDM":"a","ZBMC":"Unknown","TXDM":"99"})];
        assert!(build_answers(&questionnaire(), "s", &q, &[], &input()).is_err());
        let mut i = input();
        i.ids.push("q".into());
        assert!(i.validate().is_err());
    }
    #[test]
    fn drafts_are_bound_to_account_session_and_time() {
        let mut d = Draft {
            created: Instant::now(),
            account_id: "a".into(),
            generation: 3,
            courses: vec![],
        };
        assert!(validate_draft(&d, "a", 3).is_ok());
        assert!(validate_draft(&d, "b", 3).is_err());
        assert!(validate_draft(&d, "a", 4).is_err());
        d.created = Instant::now() - TTL;
        assert!(validate_draft(&d, "a", 3).is_err());
    }
    #[test]
    fn malformed_envelope_is_not_an_empty_success() {
        assert!(super::super::transport::rows(&json!({"code":"0","datas":{}}), "cxdwpj").is_err());
        assert!(super::super::transport::rows(
            &json!({"code":"1","datas":{"cxdwpj":{"rows":[]}}}),
            "cxdwpj"
        )
        .is_err());
    }
}
