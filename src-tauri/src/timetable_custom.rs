//! Local timetable edits: per-account, per-semester overrides of school
//! courses plus courses the user adds themselves.
//!
//! School courses have no identifier of their own, so each one is keyed by a
//! digest of its original fields. An override therefore survives a refresh for
//! as long as the school keeps that entry unchanged. Nothing here is sent to
//! the school system, and the cached copy of the raw timetable stays raw so a
//! reset can always restore the school's own values.
use crate::{campus, config::write_atomic};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

const MAX_TEXT: usize = 80;
const MAX_SEMESTER: usize = 128;
const MAX_WEEKS: usize = 60;
const MAX_WEEK: u8 = 60;
const MAX_PERIOD: u8 = 20;
const MAX_COLOR: u8 = 5;
const MAX_ID: usize = 64;
/// Both lists together; a timetable never comes close to this.
const MAX_ENTRIES: usize = 500;
const CUSTOM_PREFIX: &str = "custom-";

#[derive(Default, Serialize, Deserialize)]
struct Store {
    #[serde(default = "store_version")]
    version: u32,
    /// Campus account scope, then semester label, then that semester's edits.
    #[serde(default)]
    scopes: BTreeMap<String, BTreeMap<String, Plan>>,
}

fn store_version() -> u32 {
    1
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Plan {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) overrides: BTreeMap<String, Override>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) custom: Vec<Entry>,
}

impl Plan {
    fn len(&self) -> usize {
        self.overrides.len() + self.custom.len()
    }
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Override {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) teacher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) day: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) start: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) end: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) weeks: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) color: Option<u8>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) hidden: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Entry {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) teacher: String,
    #[serde(default)]
    pub(crate) location: String,
    pub(crate) day: u8,
    pub(crate) start: u8,
    pub(crate) end: u8,
    pub(crate) weeks: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) color: Option<u8>,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Action {
    Save,
    Hide,
    Reset,
}

/// One edit, shared by the page command and the Agent tools.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Request {
    pub(crate) semester: String,
    pub(crate) action: Action,
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) course: Option<Draft>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Draft {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) teacher: String,
    #[serde(default)]
    pub(crate) location: String,
    pub(crate) day: u8,
    pub(crate) start: u8,
    pub(crate) end: u8,
    pub(crate) weeks: Weeks,
    #[serde(default)]
    pub(crate) color: Option<u8>,
}

/// The page sends text ("1-16"); the Agent may send either form.
#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum Weeks {
    Text(String),
    List(Vec<u8>),
}

impl Draft {
    fn clean(self) -> Result<Entry, String> {
        let name = clean_text(&self.name, "课程名称")?;
        let teacher = clean_optional(&self.teacher, "教师")?;
        let location = clean_optional(&self.location, "地点")?;
        if !(1..=7).contains(&self.day) {
            return Err("星期必须在 1 到 7 之间".into());
        }
        if self.start == 0 || self.end < self.start || self.end > MAX_PERIOD {
            return Err("节次必须在 1 到 20 之间，且结束不早于开始".into());
        }
        let mut weeks = match &self.weeks {
            Weeks::Text(text) => campus::parse_weeks(text)?,
            Weeks::List(list) => list.clone(),
        };
        if weeks.is_empty() || weeks.len() > MAX_WEEKS {
            return Err("周次不能为空，且不能超过 60 个".into());
        }
        if weeks.iter().any(|week| *week == 0 || *week > MAX_WEEK) {
            return Err("周次必须在 1 到 60 之间".into());
        }
        weeks.sort_unstable();
        weeks.dedup();
        if self.color.is_some_and(|color| color > MAX_COLOR) {
            return Err("颜色编号超出范围".into());
        }
        Ok(Entry {
            id: String::new(),
            name,
            teacher,
            location,
            day: self.day,
            start: self.start,
            end: self.end,
            weeks,
            color: self.color,
        })
    }
}

fn clean_text(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{label}不能为空"));
    }
    if value.chars().count() > MAX_TEXT {
        return Err(format!("{label}不能超过 {MAX_TEXT} 个字"));
    }
    Ok(value.to_string())
}

fn clean_optional(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim();
    if value.chars().count() > MAX_TEXT {
        return Err(format!("{label}不能超过 {MAX_TEXT} 个字"));
    }
    Ok(value.to_string())
}

pub(crate) fn clean_semester(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > MAX_SEMESTER {
        return Err("学期信息无效".into());
    }
    Ok(value.to_string())
}

fn check_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > MAX_ID
        || !id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err("课程编号无效".into());
    }
    Ok(())
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|_| "无法定位课表数据目录")?
        .join("timetable-custom.json"))
}

fn load_at(path: &Path) -> Result<Store, String> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|_| "课表自定义数据无法读取，请检查 timetable-custom.json".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Store::default()),
        Err(_) => Err("无法读取课表自定义数据".into()),
    }
}

fn save_at(path: &Path, store: &Store) -> Result<(), String> {
    write_atomic(
        path,
        &serde_json::to_vec_pretty(store).map_err(|_| "无法序列化课表自定义数据")?,
    )
}

pub(crate) fn plan(app: &AppHandle, scope: &str, semester: &str) -> Result<Plan, String> {
    let store = load_at(&store_path(app)?)?;
    Ok(store
        .scopes
        .get(scope)
        .and_then(|semesters| semesters.get(semester))
        .cloned()
        .unwrap_or_default())
}

/// Applies one edit and returns the semester's plan after it.
pub(crate) fn edit(app: &AppHandle, scope: &str, request: &Request) -> Result<Plan, String> {
    let semester = clean_semester(&request.semester)?;
    let path = store_path(app)?;
    let mut store = load_at(&path)?;
    let mut plan = store
        .scopes
        .get(scope)
        .and_then(|semesters| semesters.get(&semester))
        .cloned()
        .unwrap_or_default();

    match request.action {
        Action::Save => {
            let entry = request.course.clone().ok_or("缺少课程内容")?.clean()?;
            if request.id.is_empty() {
                let id = next_custom_id(&plan);
                plan.custom.push(Entry { id, ..entry });
            } else if request.id.starts_with(CUSTOM_PREFIX) {
                let target = plan
                    .custom
                    .iter_mut()
                    .find(|item| item.id == request.id)
                    .ok_or("这门自建课程不存在，可能已被删除")?;
                *target = Entry {
                    id: target.id.clone(),
                    ..entry
                };
            } else {
                check_id(&request.id)?;
                plan.overrides
                    .insert(request.id.clone(), Override::from(entry));
            }
        }
        Action::Hide => {
            check_id(&request.id)?;
            if request.id.starts_with(CUSTOM_PREFIX) {
                plan.custom.retain(|item| item.id != request.id);
            } else {
                plan.overrides.entry(request.id.clone()).or_default().hidden = true;
            }
        }
        Action::Reset => {
            check_id(&request.id)?;
            plan.custom.retain(|item| item.id != request.id);
            plan.overrides.remove(&request.id);
        }
    }

    if plan.len() > MAX_ENTRIES {
        return Err("课表自定义条目过多".into());
    }
    if plan.is_empty() {
        if let Some(semesters) = store.scopes.get_mut(scope) {
            semesters.remove(&semester);
        }
    } else {
        store
            .scopes
            .entry(scope.to_string())
            .or_default()
            .insert(semester, plan.clone());
    }
    store.version = store_version();
    save_at(&path, &store)?;
    Ok(plan)
}

/// Drops every edit of one account, used when that account is removed.
pub(crate) fn purge(app: &AppHandle, scope: &str) -> Result<(), String> {
    let path = store_path(app)?;
    let mut store = load_at(&path)?;
    if store.scopes.remove(scope).is_some() {
        save_at(&path, &store)?;
    }
    Ok(())
}

fn next_custom_id(plan: &Plan) -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or_default();
    let mut candidate = format!("{CUSTOM_PREFIX}{stamp}");
    let mut suffix = 1;
    while plan.custom.iter().any(|item| item.id == candidate) {
        candidate = format!("{CUSTOM_PREFIX}{stamp}-{suffix}");
        suffix += 1;
    }
    candidate
}

impl From<Entry> for Override {
    fn from(entry: Entry) -> Self {
        Override {
            name: Some(entry.name),
            teacher: Some(entry.teacher),
            location: Some(entry.location),
            day: Some(entry.day),
            start: Some(entry.start),
            end: Some(entry.end),
            weeks: Some(entry.weeks),
            color: entry.color,
            hidden: false,
        }
    }
}

/// Identity of a school course, computed from the fields the school sent.
pub(crate) fn course_key(course: &campus::Course) -> String {
    let seed = format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{:?}",
        course.name,
        course.teacher,
        course.location,
        course.day,
        course.start,
        course.end,
        course.weeks
    );
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in seed.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Rewrites a raw timetable in place: overrides applied, hidden entries moved
/// aside, then the user's own courses appended.
pub(crate) fn apply(timetable: &mut campus::Timetable, plan: &Plan) {
    let mut courses = Vec::with_capacity(timetable.courses.len() + plan.custom.len());
    let mut hidden = Vec::new();
    for mut course in timetable.courses.drain(..) {
        let id = course_key(&course);
        let mut is_hidden = false;
        if let Some(over) = plan.overrides.get(&id) {
            is_hidden = over.hidden;
            over.apply_to(&mut course);
        }
        course.id = id;
        course.custom = false;
        course.edited = plan.overrides.contains_key(&course.id);
        if is_hidden {
            hidden.push(course);
        } else {
            courses.push(course);
        }
    }
    for entry in &plan.custom {
        courses.push(campus::Course {
            id: entry.id.clone(),
            name: entry.name.clone(),
            teacher: entry.teacher.clone(),
            location: entry.location.clone(),
            day: entry.day,
            start: entry.start,
            end: entry.end,
            weeks: entry.weeks.clone(),
            color: entry.color,
            custom: true,
            edited: false,
        });
    }
    timetable.courses = courses;
    timetable.hidden = hidden;
}

impl Override {
    fn apply_to(&self, course: &mut campus::Course) {
        if let Some(value) = &self.name {
            course.name = value.clone();
        }
        if let Some(value) = &self.teacher {
            course.teacher = value.clone();
        }
        if let Some(value) = &self.location {
            course.location = value.clone();
        }
        if let Some(value) = self.day {
            course.day = value;
        }
        if let Some(value) = self.start {
            course.start = value;
        }
        if let Some(value) = self.end {
            course.end = value;
        }
        if let Some(value) = &self.weeks {
            course.weeks = value.clone();
        }
        if self.color.is_some() {
            course.color = self.color;
        }
    }
}

/// Text form for the Agent: which school entries are edited and how, plus the
/// user's own courses. The raw timetable supplies the original labels.
pub(crate) fn describe(raw: &campus::Timetable, plan: &Plan) -> Value {
    let originals: BTreeMap<String, Value> = raw
        .courses
        .iter()
        .map(|course| {
            (
                course_key(course),
                json!({
                    "name": course.name,
                    "teacher": course.teacher,
                    "location": course.location,
                    "day": course.day,
                    "start": course.start,
                    "end": course.end,
                    "weeks": course.weeks,
                }),
            )
        })
        .collect();
    let overrides: Vec<Value> = plan
        .overrides
        .iter()
        .map(|(id, change)| {
            json!({
                "id": id,
                "original": originals.get(id),
                "changes": change,
            })
        })
        .collect();
    json!({
        "semester": raw.semester,
        "overrides": overrides,
        "custom": plan.custom,
    })
}

/// What one request would change, without writing anything.
pub(crate) fn preview(
    raw: &campus::Timetable,
    plan: &Plan,
    request: &Request,
) -> Result<Value, String> {
    let mut merged = raw.clone();
    apply(&mut merged, plan);
    let before = merged
        .courses
        .iter()
        .find(|course| course.id == request.id)
        .map(course_summary);
    let after = match request.action {
        Action::Save => {
            let entry = request.course.clone().ok_or("缺少课程内容")?.clean()?;
            Some(course_summary(&campus::Course {
                id: request.id.clone(),
                name: entry.name,
                teacher: entry.teacher,
                location: entry.location,
                day: entry.day,
                start: entry.start,
                end: entry.end,
                weeks: entry.weeks,
                color: entry.color,
                custom: request.id.starts_with(CUSTOM_PREFIX),
                edited: false,
            }))
        }
        Action::Hide | Action::Reset => None,
    };
    if !request.id.is_empty() && request.action != Action::Save && before.is_none() {
        // Reset may target a hidden school course, which is absent from the merged grid.
        let hidden = merged
            .hidden
            .iter()
            .find(|course| course.id == request.id)
            .map(course_summary);
        if hidden.is_none() {
            return Err("找不到要修改的课程，请重新查询课表".into());
        }
    }
    let message = match request.action {
        Action::Save if request.id.is_empty() => "将新增一门只有本机可见的课程。",
        Action::Save => "将修改这门课在本机的显示。",
        Action::Hide => "将把这门课从本机课表中隐藏。",
        Action::Reset => "将恢复这门课在本机的默认状态。",
    };
    Ok(json!({
        "confirmRequired": true,
        "semester": raw.semester,
        "action": request.action,
        "before": before,
        "after": after,
        "message": message,
    }))
}

pub(crate) fn course_summary(course: &campus::Course) -> Value {
    json!({
        "id": course.id,
        "name": course.name,
        "teacher": course.teacher,
        "location": course.location,
        "day": course.day,
        "start": course.start,
        "end": course.end,
        "weeks": course.weeks,
        "color": course.color,
        "custom": course.custom,
        "edited": course.edited,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn course(name: &str) -> campus::Course {
        campus::Course {
            id: String::new(),
            name: name.into(),
            teacher: "张老师".into(),
            location: "主楼 A203".into(),
            day: 1,
            start: 1,
            end: 2,
            weeks: vec![1, 2, 3],
            color: None,
            custom: false,
            edited: false,
        }
    }

    fn timetable(courses: Vec<campus::Course>) -> campus::Timetable {
        campus::Timetable {
            semester: "2026-2027 · 秋季".into(),
            start_date: "2026-09-07".into(),
            end_date: "2027-01-10".into(),
            courses,
            hidden: Vec::new(),
        }
    }

    fn draft() -> Draft {
        Draft {
            name: " 自习 ".into(),
            teacher: String::new(),
            location: " 图书馆 ".into(),
            day: 3,
            start: 3,
            end: 4,
            weeks: Weeks::Text("1-4，6".into()),
            color: Some(2),
        }
    }

    fn request(action: Action, id: &str) -> Request {
        Request {
            semester: "2026-2027 · 秋季".into(),
            action,
            id: id.into(),
            course: None,
        }
    }

    #[test]
    fn course_keys_are_stable_and_distinct() {
        let first = course("高等数学");
        assert_eq!(course_key(&first), course_key(&first.clone()));
        let mut moved = course("高等数学");
        moved.day = 2;
        assert_ne!(course_key(&first), course_key(&moved));
        assert_eq!(course_key(&first).len(), 16);
    }

    #[test]
    fn draft_is_trimmed_and_validated() {
        let entry = draft().clean().unwrap();
        assert_eq!(entry.name, "自习");
        assert_eq!(entry.location, "图书馆");
        assert_eq!(entry.weeks, vec![1, 2, 3, 4, 6]);

        let mut blank = draft();
        blank.name = "   ".into();
        assert!(blank.clean().is_err());

        let mut backwards = draft();
        backwards.end = backwards.start - 1;
        assert!(backwards.clean().is_err());

        let mut odd = draft();
        odd.day = 8;
        assert!(odd.clean().is_err());

        let mut bad_weeks = draft();
        bad_weeks.weeks = Weeks::List(vec![0]);
        assert!(bad_weeks.clean().is_err());

        let mut bad_color = draft();
        bad_color.color = Some(6);
        assert!(bad_color.clean().is_err());
    }

    #[test]
    fn overrides_rename_and_hide_school_courses() {
        let mut table = timetable(vec![course("高等数学"), course("大学物理")]);
        let math = course_key(&table.courses[0]);
        let physics = course_key(&table.courses[1]);

        let change = Entry {
            id: String::new(),
            name: "高数（改）".into(),
            teacher: "李老师".into(),
            location: "主楼 B101".into(),
            day: 4,
            start: 5,
            end: 6,
            weeks: vec![1, 2],
            color: Some(1),
        }
        .into();
        let mut plan = Plan::default();
        plan.overrides.insert(math.clone(), change);

        let hidden = Override {
            hidden: true,
            ..Default::default()
        };
        plan.overrides.insert(physics.clone(), hidden);

        apply(&mut table, &plan);
        assert_eq!(table.courses.len(), 1);
        assert_eq!(table.hidden.len(), 1);
        let edited = &table.courses[0];
        assert_eq!(edited.id, math);
        assert_eq!(edited.name, "高数（改）");
        assert_eq!(edited.day, 4);
        assert_eq!(edited.color, Some(1));
        assert!(edited.edited);

        // Re-applying to raw data gives the same answer, so a refresh is safe.
        let mut fresh = timetable(vec![course("高等数学"), course("大学物理")]);
        apply(&mut fresh, &plan);
        assert_eq!(fresh.courses.len(), 1);
        assert_eq!(fresh.courses[0].name, "高数（改）");
    }

    #[test]
    fn custom_courses_are_appended_and_marked() {
        let mut table = timetable(vec![course("高等数学")]);
        let mut plan = Plan::default();
        plan.custom.push(Entry {
            id: "custom-1".into(),
            name: "自习".into(),
            teacher: String::new(),
            location: "图书馆".into(),
            day: 3,
            start: 3,
            end: 4,
            weeks: vec![1],
            color: None,
        });
        apply(&mut table, &plan);
        assert_eq!(table.courses.len(), 2);
        assert!(table.courses[1].custom);
        assert_eq!(table.courses[1].id, "custom-1");
    }

    #[test]
    fn preview_never_writes_and_reports_the_target() {
        let table = timetable(vec![course("高等数学")]);
        let id = course_key(&table.courses[0]);
        let mut edit = request(Action::Save, &id);
        edit.course = Some(draft());
        let answer = preview(&table, &Plan::default(), &edit).unwrap();
        assert_eq!(answer["confirmRequired"], true);
        assert_eq!(answer["before"]["name"], "高等数学");
        assert_eq!(answer["after"]["name"], "自习");

        let missing = preview(&table, &Plan::default(), &request(Action::Hide, "ffff"));
        assert!(missing.is_err());
    }

    #[test]
    fn edit_round_trips_through_a_file() {
        let dir = std::env::temp_dir().join(format!("xjtu-timetable-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("timetable-custom.json");

        let mut store = Store::default();
        let semester = "2026-2027 · 秋季";
        let mut plan = Plan::default();
        let mut add = request(Action::Save, "");
        add.course = Some(draft());
        // Writes go through `edit`, so exercise the pieces it uses directly.
        let entry = add.course.clone().unwrap().clean().unwrap();
        plan.custom.push(Entry {
            id: next_custom_id(&plan),
            ..entry
        });
        store
            .scopes
            .entry("scope".into())
            .or_default()
            .insert(semester.into(), plan.clone());
        save_at(&path, &store).unwrap();

        let loaded = load_at(&path).unwrap();
        let stored = loaded.scopes["scope"][semester].clone();
        assert_eq!(stored.custom.len(), 1);
        assert_eq!(stored.custom[0].weeks, vec![1, 2, 3, 4, 6]);

        // A missing file is an empty plan, not an error.
        assert!(load_at(&dir.join("missing.json"))
            .unwrap()
            .scopes
            .is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hidden_school_courses_can_be_described_and_reset() {
        let raw = timetable(vec![course("高等数学")]);
        let id = course_key(&raw.courses[0]);
        let mut plan = Plan::default();
        plan.overrides.entry(id.clone()).or_default().hidden = true;
        let described = describe(&raw, &plan);
        assert_eq!(described["overrides"][0]["id"], id.as_str());
        assert_eq!(described["overrides"][0]["original"]["name"], "高等数学");

        let mut merged = raw.clone();
        apply(&mut merged, &plan);
        assert!(merged.courses.is_empty());
        let preview = preview(&raw, &plan, &request(Action::Reset, &id)).unwrap();
        assert_eq!(preview["message"], "将恢复这门课在本机的默认状态。");
    }
}
