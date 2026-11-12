//! Fitness test scores, measurement details, appointments and exemption.
//!
//! Item numbers 1–7 match the platform: 1 height/weight, 2 vital capacity,
//! 3 standing long jump, 4 sit-and-reach, 5 pull-up / sit-up, 6 50 m,
//! 7 800 / 1000 m.
use super::{data, text, Session};
use serde_json::{json, Value};

const YEARS: &str = "bdlp_h5_fitness_test/public/index.php/index/fitness/fitnessYear";
const SCORE: &str = "bdlp_h5_fitness_test/public/index.php/index/Report/getStudentScore";
const ITEM: &str = "bdlp_h5_fitness_test/public/index.php/index/Report/getItemDetails";
const EYES: &str = "bdlp_h5_fitness_test/public/index.php/index/Report/getEyeDetails";
const NOTICES: &str = "bdlp_h5_fitness_test/public/index.php/index/News/getSchoolNoticeList";
const PLAN: &str = "bdlp_h5_fitness_test/public/index.php/index/fitness/getPlan";
const PLAN_BY_DATE: &str = "bdlp_h5_fitness_test/public/index.php/index/fitness/getPlanByDate";
const CLASS_DETAIL: &str =
    "bdlp_h5_fitness_test/public/index.php/index/Appoint/getSchoolFitClassDetail";
const CHECK_APPOINT: &str = "bdlp_h5_fitness_test/public/index.php/index/Appoint/checkAppoint";
const APPOINT_TIPS: &str = "bdlp_h5_fitness_test/public/index.php/index/appoint/tips";
const APPOINT: &str = "bdlp_h5_fitness_test/public/index.php/index/Appoint/setOnAppointment";
const CANCEL_APPOINT: &str =
    "bdlp_h5_fitness_test/public/index.php/index/Appoint/setOffAppointment";
const FREE_YEARS: &str = "bdlp_h5_fitness_test/public/index.php/index/Free/getYear";
const APPLY_FREE: &str = "bdlp_h5_fitness_test/public/index.php/index/Free/setFree";

const ITEMS: [(&str, &str); 7] = [
    ("bmi", "身高体重"),
    ("vc", "肺活量"),
    ("jump", "立定跳远"),
    ("sit_and_reach", "坐位体前屈"),
    ("pull_and_sit", "引体向上 / 仰卧起坐"),
    ("50m", "50 米"),
    ("run", "800 / 1000 米"),
];

pub(crate) struct Client<'a> {
    pub(super) session: &'a Session,
}

impl Client<'_> {
    /// Years that have a fitness report.
    pub(crate) async fn years(&self) -> Result<Vec<Value>, String> {
        let data = data(&self.session.post(YEARS, &[("from", "1")]).await?);
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

    /// Total score and the seven item scores for one year.
    pub(crate) async fn score(&self, year: &str) -> Result<Value, String> {
        let data = data(&self.session.post(SCORE, &[("year_num", year)]).await?);
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
                    "converted": text(data.get(format!("{key}_score2"))).unwrap_or_default(),
                    "grade": text(data.get(format!("{key}_grade"))).unwrap_or_default(),
                    "class": text(data.get(format!("{key}_class"))).unwrap_or_default(),
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

    /// Raw measurements and history for one item.
    ///
    /// The reply keeps the platform shape: `info.value_1`, `info.value_2`,
    /// `info.photo_url` (body-composition chart for item 1) and `list`.
    pub(crate) async fn item(&self, item: u8, year: &str) -> Result<Value, String> {
        if !(1..=7).contains(&item) {
            return Err("体测项目编号只能是 1 到 7".into());
        }
        let item_num = item.to_string();
        let value = self
            .session
            .post(ITEM, &[("item_num", &item_num), ("year_num", year)])
            .await?;
        Ok(data(&value))
    }

    /// Vision records for one year. `from` and `student_num` are only needed
    /// when an administrator reads another student's record.
    pub(crate) async fn eyes(
        &self,
        year: &str,
        from: Option<&str>,
        student_num: Option<&str>,
    ) -> Result<Value, String> {
        let value = self
            .session
            .post(
                EYES,
                &[
                    ("year_num", year),
                    ("from", from.unwrap_or("")),
                    ("student_num", student_num.unwrap_or("")),
                ],
            )
            .await?;
        Ok(data(&value))
    }

    /// Sports notices. `notice_type` is 1 for the list page's first tab and
    /// 2 for the second; `kind` maps to the page's optional `type` filter.
    pub(crate) async fn notices(
        &self,
        page: u32,
        notice_type: u8,
        kind: Option<&str>,
    ) -> Result<Value, String> {
        let page = page.to_string();
        let notice_type = notice_type.to_string();
        let value = self
            .session
            .post(
                NOTICES,
                &[
                    ("page", &page),
                    ("notice_type", &notice_type),
                    ("type", kind.unwrap_or("")),
                ],
            )
            .await?;
        Ok(data(&value))
    }

    /// Appointment plan, venue sessions and blacklist status.
    pub(crate) async fn plan(&self) -> Result<Value, String> {
        let value = self.session.post(PLAN, &[]).await?;
        Ok(data(&value))
    }

    /// Bookable slots for a date. `date = "2099-01-01"` asks the platform for
    /// the earliest available day, which is how its own page starts.
    pub(crate) async fn plan_by_date(
        &self,
        date: &str,
        plan_id: &str,
        location_id: &str,
    ) -> Result<Value, String> {
        let value = self
            .session
            .post(
                PLAN_BY_DATE,
                &[
                    ("date", date),
                    ("plan_id", plan_id),
                    ("location_id", location_id),
                ],
            )
            .await?;
        Ok(data(&value))
    }

    /// Detail shown before booking or cancelling an appointment.
    pub(crate) async fn class_detail(
        &self,
        class_id: &str,
        class_time: &str,
        test_time: &str,
        appoint_id: Option<&str>,
    ) -> Result<Value, String> {
        let value = self
            .session
            .post(
                CLASS_DETAIL,
                &[
                    ("class_id", class_id),
                    ("class_time", class_time),
                    ("test_time", test_time),
                    ("appoint_id", appoint_id.unwrap_or("")),
                ],
            )
            .await?;
        Ok(data(&value))
    }

    /// Pre-flight check. It does not create an appointment.
    pub(crate) async fn check_appointment(
        &self,
        class_id: &str,
        class_time: &str,
        location_id: &str,
    ) -> Result<Value, String> {
        let value = self
            .session
            .post(
                CHECK_APPOINT,
                &[
                    ("class_id", class_id),
                    ("class_time", class_time),
                    ("location_id", location_id),
                ],
            )
            .await?;
        Ok(data(&value))
    }

    pub(crate) async fn appointment_tips(&self, school_id: &str) -> Result<Value, String> {
        let value = self
            .session
            .post(APPOINT_TIPS, &[("school_id", school_id)])
            .await?;
        Ok(data(&value))
    }

    /// Write: caller must preview the slot and obtain explicit confirmation.
    pub(crate) async fn appoint(
        &self,
        class_id: &str,
        class_time: &str,
        location_id: &str,
    ) -> Result<Value, String> {
        let value = self
            .session
            .post(
                APPOINT,
                &[
                    ("class_id", class_id),
                    ("class_time", class_time),
                    ("location_id", location_id),
                ],
            )
            .await?;
        Ok(data(&value))
    }

    /// Write: cancelling cannot be undone. Do not retry when the result is
    /// unknown; ask the user to check the booking page instead.
    pub(crate) async fn cancel_appointment(
        &self,
        class_id: &str,
        class_time: &str,
        location_id: &str,
    ) -> Result<Value, String> {
        let value = self
            .session
            .post(
                CANCEL_APPOINT,
                &[
                    ("class_id", class_id),
                    ("class_time", class_time),
                    ("location_id", location_id),
                ],
            )
            .await?;
        Ok(data(&value))
    }

    /// Exemption application options: current year, allowed types and flags
    /// such as whether BMI must be filled in.
    pub(crate) async fn free_years(&self) -> Result<Value, String> {
        let value = self.session.post(FREE_YEARS, &[]).await?;
        Ok(json!({
            "year": data(&value),
            "freeTypes": value.get("freeType").cloned().unwrap_or(Value::Null),
            "miance": value.get("miance").cloned().unwrap_or(Value::Null),
            "hideBmi": value.get("hideBmi").cloned().unwrap_or(Value::Null),
        }))
    }

    /// Write: `photo` is the uploaded file name returned by the platform's
    /// upload step, not a local path. Preview first, confirm before sending.
    pub(crate) async fn apply_free(
        &self,
        free_type: &str,
        describe: &str,
        photo: &str,
        item: &str,
        stype: Option<&str>,
    ) -> Result<Value, String> {
        let mut form = vec![
            ("free_type", free_type),
            ("describe", describe),
            ("photo", photo),
            ("item", item),
        ];
        if let Some(stype) = stype {
            form.push(("stype", stype));
        }
        let value = self.session.post(APPLY_FREE, &form).await?;
        Ok(data(&value))
    }
}
