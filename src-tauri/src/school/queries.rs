use super::transport::{condition, field, Client};
use crate::{
    accounts::Account,
    network::{self, AccessMode, Transport},
};
use scraper::{Html, Selector};
use serde::Deserialize;
use serde_json::{json, Value};
use url::Url;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Query {
    Grades {
        term: Option<String>,
    },
    Exams {
        term: Option<String>,
    },
    Terms,
    Evaluations {
        term: Option<String>,
        finished: bool,
    },
    RoomOptions,
    Rooms {
        campus: String,
        building: String,
        date: String,
        start: u8,
        end: u8,
    },
    Courses {
        term: Option<String>,
        name: String,
        campus: String,
    },
    Calendar,
    AttendanceStreams {
        start: Option<String>,
        end: Option<String>,
        page: u32,
    },
    AttendanceRecords {
        start: String,
        end: String,
        page: u32,
    },
    Card,
    CardTransactions {
        start: String,
        end: String,
        page: u32,
    },
    Fitness,
    FitnessScore {
        year: String,
    },
    Profile,
    LmsCourses,
    LmsActivities {
        course: u64,
    },
    LmsActivity {
        activity: u64,
    },
    LmsSubmissions {
        activity: u64,
    },
    NoticeChannels,
    Notices {
        channel: String,
        limit: u32,
    },
    LibraryFloors,
    LibraryAreas {
        floor: String,
    },
    LibrarySeats {
        area: String,
    },
    VenueList,
    VenueSlots {
        venue: u64,
        date: String,
    },
    VenueOrders,
}
impl Query {
    pub fn feature(&self) -> &'static str {
        match self {
            Self::Grades { .. } => "grades",
            Self::Exams { .. } => "exams",
            Self::Terms | Self::Courses { .. } => "courses",
            Self::Evaluations { .. } => "evaluations",
            Self::RoomOptions | Self::Rooms { .. } => "rooms",
            Self::Calendar => "calendar",
            Self::AttendanceStreams { .. } | Self::AttendanceRecords { .. } => "attendance",
            Self::Card | Self::CardTransactions { .. } => "card",
            Self::Fitness | Self::FitnessScore { .. } => "fitness",
            Self::Profile => "profile",
            Self::LmsCourses
            | Self::LmsActivities { .. }
            | Self::LmsActivity { .. }
            | Self::LmsSubmissions { .. } => "lms",
            Self::NoticeChannels | Self::Notices { .. } => "notices",
            Self::LibraryFloors | Self::LibraryAreas { .. } | Self::LibrarySeats { .. } => {
                "library"
            }
            Self::VenueList | Self::VenueSlots { .. } | Self::VenueOrders => "venues",
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Grades { term }
            | Self::Exams { term }
            | Self::Evaluations { term, .. }
            | Self::Courses { term, .. } => {
                if let Some(term) = term {
                    validate_term(term)?;
                }
            }
            Self::Rooms {
                campus,
                building,
                date,
                start,
                end,
            } if campus.len() > 128
                || building.len() > 128
                || !valid_date(date)
                || *start == 0
                || end < start
                || *end > 14 =>
            {
                return Err("请选择有效校区、日期和节次范围".into());
            }
            Self::AttendanceStreams { start, end, page }
                if !(1..=500).contains(page)
                    || start.as_deref().is_some_and(|date| !valid_date(date))
                    || end.as_deref().is_some_and(|date| !valid_date(date))
                    || matches!((start, end), (Some(start), Some(end)) if start > end) =>
            {
                return Err("请选择有效的日期范围和页码".into());
            }
            Self::AttendanceRecords { start, end, page }
                if !(1..=500).contains(page)
                    || !valid_date(start)
                    || !valid_date(end)
                    || start > end =>
            {
                return Err("请选择有效的日期范围和页码".into());
            }
            Self::CardTransactions { start, end, page }
                if !(1..=200).contains(page)
                    || !valid_date(start)
                    || !valid_date(end)
                    || start > end =>
            {
                return Err("请选择有效的日期范围和页码".into());
            }
            Self::FitnessScore { year }
                if year.len() != 4 || !year.bytes().all(|byte| byte.is_ascii_digit()) =>
            {
                return Err("请选择有效的体测学年".into());
            }
            Self::LmsActivities { course } if !(1..=9_999_999).contains(course) => {
                return Err("课程编号无效".into());
            }
            Self::LmsActivity { activity } | Self::LmsSubmissions { activity }
                if !(1..=9_999_999_999).contains(activity) =>
            {
                return Err("活动编号无效".into());
            }
            Self::Notices { channel, limit } => super::notices::validate(channel, *limit)?,
            Self::LibraryAreas { floor } if !super::library::valid_floor(floor) => {
                return Err("请选择有效的图书馆楼层".into());
            }
            Self::LibrarySeats { area } if !super::library::valid_area(area) => {
                return Err("请选择有效的图书馆区域".into());
            }
            Self::VenueSlots { venue, date }
                if !(1..=9_999_999).contains(venue) || !valid_date(date) =>
            {
                return Err("请选择有效的场馆和日期".into());
            }
            _ => {}
        }
        if let Self::Courses { name, campus, .. } = self {
            if name.len() > 256 || campus.len() > 128 {
                return Err("课程查询条件过长".into());
            }
        }
        Ok(())
    }
}
fn validate_term(term: &str) -> Result<(), String> {
    let parts: Vec<_> = term.split('-').collect();
    if parts.len() != 3
        || parts[0].len() != 4
        || parts[1].len() != 4
        || !parts.iter().all(|p| p.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("学期格式应为 2026-2027-1".into());
    }
    let year: u32 = parts[0].parse().map_err(|_| "学期格式无效")?;
    if parts[1].parse::<u32>().ok() != Some(year + 1) || !matches!(parts[2], "1" | "2" | "3" | "4")
    {
        return Err("学期年份或编号无效".into());
    }
    Ok(())
}
pub(super) fn valid_date(value: &str) -> bool {
    let p: Vec<_> = value.split('-').collect();
    if p.len() != 3 || p[0].len() != 4 || p[1].len() != 2 || p[2].len() != 2 {
        return false;
    }
    let (Ok(y), Ok(m), Ok(d)) = (
        p[0].parse::<u32>(),
        p[1].parse::<u32>(),
        p[2].parse::<u32>(),
    ) else {
        return false;
    };
    let max = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if y % 400 == 0 || y % 4 == 0 && y % 100 != 0 {
                29
            } else {
                28
            }
        }
        _ => 0,
    };
    (2000..=2200).contains(&y) && d > 0 && d <= max
}
pub async fn query(
    account: Option<Account>,
    query: Query,
) -> Result<(Value, Option<String>), String> {
    query.validate()?;
    if matches!(query, Query::Calendar) {
        return calendar().await.map(|v| (v, None));
    }
    if matches!(query, Query::LibraryFloors) {
        return Ok((super::library::floors(), None));
    }
    if matches!(query, Query::NoticeChannels | Query::Notices { .. }) {
        let value = match query {
            Query::NoticeChannels => super::notices::channels(),
            Query::Notices { channel, limit } => super::notices::list(&channel, limit).await?,
            _ => unreachable!(),
        };
        return Ok((value, None));
    }
    if matches!(
        query,
        Query::LibraryAreas { .. } | Query::LibrarySeats { .. }
    ) {
        let account = account.ok_or("请先登录校园账号")?;
        let client = super::library::Client::connect(&account)?;
        let value = match query {
            Query::LibraryAreas { floor } => client.areas(&floor).await?,
            Query::LibrarySeats { area } => client.seats(&area).await?,
            _ => unreachable!(),
        };
        return Ok((value, None));
    }
    if matches!(
        query,
        Query::VenueList | Query::VenueSlots { .. } | Query::VenueOrders
    ) {
        let account = account.ok_or("请先登录校园账号")?;
        let client = super::venue::Client::connect(&account)?;
        let value = match query {
            Query::VenueList => client.venues().await?,
            Query::VenueSlots { venue, date } => client.slots(venue, &date).await?,
            Query::VenueOrders => client.orders().await?,
            _ => unreachable!(),
        };
        return Ok((value, None));
    }
    if matches!(
        query,
        Query::AttendanceStreams { .. } | Query::AttendanceRecords { .. }
    ) {
        let client = super::attendance::Client::connect(account.ok_or("请先登录校园账号")?)?;
        let value = match query {
            Query::AttendanceStreams { start, end, page } => {
                client
                    .streams(start.as_deref(), end.as_deref(), page)
                    .await?
            }
            Query::AttendanceRecords { start, end, page } => {
                client.records(&start, &end, page).await?
            }
            _ => unreachable!(),
        };
        return Ok((value, None));
    }
    if matches!(query, Query::Fitness | Query::FitnessScore { .. }) {
        let account = account.ok_or("请先登录校园账号")?;
        let client = super::fitness::Client::connect(&account).await?;
        let value = match query {
            Query::Fitness => json!({"rows": client.years().await?}),
            Query::FitnessScore { year } => client.score(&year).await?,
            _ => unreachable!(),
        };
        return Ok((value, None));
    }
    if matches!(query, Query::Profile) {
        let account = account.ok_or("请先登录校园账号")?;
        return Ok((super::profile::query(&account).await?, None));
    }
    if matches!(
        query,
        Query::LmsCourses
            | Query::LmsActivities { .. }
            | Query::LmsActivity { .. }
            | Query::LmsSubmissions { .. }
    ) {
        let account = account.ok_or("请先登录校园账号")?;
        let client = super::lms::Client::connect(&account)?;
        let value = match query {
            Query::LmsCourses => client.courses().await?,
            Query::LmsActivities { course } => client.activities(course).await?,
            Query::LmsActivity { activity } => client.activity(activity).await?,
            Query::LmsSubmissions { activity } => client.submissions(activity).await?,
            _ => unreachable!(),
        };
        return Ok((value, None));
    }
    if matches!(query, Query::Card | Query::CardTransactions { .. }) {
        let account = account.ok_or("请先登录校园账号")?;
        let client = super::card::Client::connect(&account).await?;
        let value = match query {
            Query::Card => client.info().await?,
            Query::CardTransactions { start, end, page } => {
                client.transactions(&start, &end, page).await?
            }
            _ => unreachable!(),
        };
        return Ok((value, None));
    }
    let client = Client::connect(account.ok_or("请先登录校园账号")?).await?;
    let value = match query {
        Query::Grades { term } => {
            let mut filters = vec![condition("SFYX", "1")];
            if let Some(t) = term {
                filters.push(condition("XNXQDM", &t));
            }
            let rows = client
                .paged(
                    "/jwapp/sys/cjcx/modules/cjcx/xscjcx.do",
                    "xscjcx",
                    &[("querySetting".into(), json!(filters).to_string())],
                )
                .await?;
            json!({"rows":rows.iter().map(grade).collect::<Vec<_>>()})
        }
        Query::Exams { term } => {
            let term = term_or_current(&client, term).await?;
            let rows = client
                .paged(
                    "/jwapp/sys/studentWdksapApp/modules/wdksap/wdksap.do",
                    "wdksap",
                    &[
                        ("XNXQDM".into(), term.clone()),
                        ("*order".into(), "-KSRQ,-KSSJMS".into()),
                    ],
                )
                .await?;
            json!({"term":term,"rows":rows.iter().map(|r| json!({"course":field(r,"KCM"),"date":field(r,"KSRQ"),"time":field(r,"KSSJMS"),"location":field(r,"JASMC"),"seat":field(r,"ZWH"),"campus":field(r,"XXXQDM_DISPLAY")})).collect::<Vec<_>>()})
        }
        Query::Terms => {
            client.initialize_courses().await?;
            let rows = client
                .rows(
                    "/jwapp/sys/kcbcx/modules/bjkcb/xnxqcx.do",
                    "xnxqcx",
                    &[("*order".into(), "-DM".into())],
                )
                .await?;
            json!({"rows":rows.iter().map(|r|json!({"id":field(r,"DM"),"name":field(r,"MC")})).collect::<Vec<_>>()})
        }
        Query::Evaluations { term, finished } => {
            json!({"rows":super::evaluation::list(&client,term,finished).await?})
        }
        Query::RoomOptions => {
            client.student_role().await?;
            let campuses = client
                .rows(
                    "/jwapp/code/83a986fc-e677-400e-99a4-c7bb39c2ca35.do",
                    "code",
                    &[],
                )
                .await?;
            let buildings = client
                .rows(
                    "/jwapp/code/551fbcc3-cf07-4566-af1e-fc7ce272ddc1.do",
                    "code",
                    &[],
                )
                .await?;
            let options = |rows: Vec<Value>| {
                rows.into_iter()
                    .map(|r| json!({"id":field(&r,"id"),"name":field(&r,"name")}))
                    .collect::<Vec<_>>()
            };
            json!({"campuses":options(campuses),"buildings":options(buildings)})
        }
        Query::Rooms {
            campus,
            building,
            date,
            start,
            end,
        } => {
            client.student_role().await?;
            let data = vec![
                ("XXXQDM".into(), campus),
                ("JXLDM".into(), building),
                ("KXRQ".into(), date),
                ("KSJC".into(), start.to_string()),
                ("JSJC".into(), end.to_string()),
            ];
            let rows = client
                .paged("/jwapp/sys/kxjas/modules/kxjscx/cxkxjs.do", "cxkxjs", &data)
                .await?;
            json!({"rows":rows.iter().filter(|r|!field(r,"JASLXDM").is_empty()&&!field(r,"JASMC").contains("测试专用")).map(|r|json!({"name":field(r,"JASMC"),"building":field(r,"JXLDM_DISPLAY"),"category":field(r,"JASLXDM_DISPLAY"),"capacity":r["SKZWS"],"campus":field(r,"XXXQDM_DISPLAY")})).collect::<Vec<_>>()})
        }
        Query::Courses { term, name, campus } => {
            client.initialize_courses().await?;
            let term = term_or_current(&client, term).await?;
            let mut filters = vec![
                condition("XNXQDM", &term),
                json!([condition("RWZTDM","1"),{"name":"RWZTDM","linkOpt":"or","builder":"isNull"}]),
            ];
            if !name.trim().is_empty() {
                let mut c = condition("KCM", name.trim());
                c["builder"] = json!("include");
                filters.push(c);
            }
            if !campus.is_empty() {
                filters.push(condition("XXXQDM", &campus));
            }
            let rows = client
                .paged(
                    "/jwapp/sys/kcbcx/modules/qxkcb/qxfbkccx.do",
                    "qxfbkccx",
                    &[("querySetting".into(), json!(filters).to_string())],
                )
                .await?;
            json!({"term":term,"rows":rows.iter().map(|r|json!({"code":field(r,"KCH"),"name":field(r,"KCM"),"class":field(r,"KXH"),"teachers":field(r,"SKJS"),"credits":r["XF"],"campus":field(r,"XXXQDM_DISPLAY"),"department":field(r,"KKDWDM_DISPLAY"),"timePlace":field(r,"YPSJDD")})).collect::<Vec<_>>()})
        }
        Query::Calendar => unreachable!(),
        Query::AttendanceStreams { .. } | Query::AttendanceRecords { .. } => unreachable!(),
        Query::Card | Query::CardTransactions { .. } => unreachable!(),
        Query::Fitness | Query::FitnessScore { .. } => unreachable!(),
        Query::Profile
        | Query::LmsCourses
        | Query::LmsActivities { .. }
        | Query::LmsActivity { .. }
        | Query::LmsSubmissions { .. }
        | Query::NoticeChannels
        | Query::Notices { .. }
        | Query::LibraryFloors
        | Query::LibraryAreas { .. }
        | Query::LibrarySeats { .. }
        | Query::VenueList
        | Query::VenueSlots { .. }
        | Query::VenueOrders => unreachable!(),
    };
    Ok((value, Some(client.transport.snapshot()?)))
}
async fn term_or_current(client: &Client, term: Option<String>) -> Result<String, String> {
    match term {
        Some(t) => Ok(t),
        None => client.term().await,
    }
}
fn number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|v| v.parse::<f64>().ok()))
        .filter(|n| n.is_finite() && *n >= 0.0)
}
fn grade(row: &Value) -> Value {
    json!({"id":field(row,"WID"),"name":field(row,"KCM"),"code":field(row,"KCH"),"term":field(row,"XNXQDM"),"credits":number(&row["XF"]),"score":field(row,"ZCJ"),"gpa":number(&row["XFJD"]),"category":field(row,"KCXZDM_DISPLAY"),"passed":match field(row,"SFJG").as_str(){"1"=>Some(true),"0"=>Some(false),_=>None}})
}
const CALENDAR: &str = "https://dean.xjtu.edu.cn/xxfw/xl.htm";
async fn calendar() -> Result<Value, String> {
    let transport = Transport::new(None)?;
    let page = transport
        .page(
            &Url::parse(CALENDAR).unwrap(),
            AccessMode::Direct,
            "undergraduate",
        )
        .await?;
    Ok(json!({"rows":calendar_images(&page.html)}))
}
fn calendar_images(html: &str) -> Vec<Value> {
    let doc = Html::parse_document(html);
    let anchor = Selector::parse("a[href]").unwrap();
    let image = Selector::parse("img").unwrap();
    let year = regex::Regex::new(r"\d{4}-\d{4}").unwrap();
    let mut seen = std::collections::HashSet::new();
    doc.select(&anchor)
        .filter(|a| a.select(&image).next().is_some())
        .filter_map(|a| {
            let mut url = Url::parse(CALENDAR)
                .ok()?
                .join(a.value().attr("href")?)
                .ok()?;
            if url.scheme() == "http" {
                url.set_scheme("https").ok()?;
            }
            if !network::campus_web_url(&url)
                || ![".jpg", ".jpeg", ".png"]
                    .iter()
                    .any(|ext| url.path().to_ascii_lowercase().ends_with(ext))
                || !seen.insert(url.to_string())
            {
                return None;
            }
            let title = a.value().attr("title").unwrap_or("");
            let label = year
                .find(title)
                .or_else(|| year.find(url.path()))
                .map(|v| v.as_str().to_string());
            Some(json!({"year":label,"url":url.as_str()}))
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_query_before_any_network() {
        assert!(validate_term("2026-2027-1").is_ok());
        for bad in ["2026-2028-1", "2026-2027-0", "../", "x-1-1"] {
            assert!(validate_term(bad).is_err());
        }
        assert!(valid_date("2024-02-29"));
        assert!(!valid_date("2026-02-29"));
        assert!(!valid_date("2026-04-31"));
    }
    #[test]
    fn grade_preserves_text_scores_and_unknowns() {
        let g = grade(&json!({"KCM":"Test","ZCJ":"A＋","XF":"2.5","XFJD":null}));
        assert_eq!(g["score"], "A＋");
        assert_eq!(g["credits"], 2.5);
        assert!(g["gpa"].is_null());
        assert!(g["passed"].is_null());
    }
    #[test]
    fn calendar_filters_links_and_never_invents_years() {
        let rows = calendar_images(
            r#"<a href='/2026-2027.jpg'><img src='thumb.png'></a><a href='/2026-2027.jpg'><img></a><a href='https://evil.test/a.png'><img></a><a href='/old.png'><img></a>"#,
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["year"], "2026-2027");
        assert!(rows[1]["year"].is_null());
    }
}
