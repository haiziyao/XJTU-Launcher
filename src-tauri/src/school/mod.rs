//! Campus business adapters. Protocol research: docs/CAMPUS_FEATURES.md on the dev branch.
//! This module never submits passwords or sends evaluation answers during a query.
mod attendance;
mod card;
pub mod evaluation;
mod fitness;
mod library;
mod lms;
mod profile;
mod queries;
mod transport;
mod venue;

/// SSO entry used when a direct session is created; the card service is the
/// most common reason to need one.
pub(crate) use card::LOGIN_URL as CARD_LOGIN;
pub(crate) use library::LOGIN_URL as LIBRARY_LOGIN;
pub(crate) use lms::LOGIN_URL as LMS_LOGIN;
pub use queries::{query, Query};
use serde::Serialize;
pub(crate) use venue::LOGIN_URL as VENUE_LOGIN;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Feature {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub backend: &'static str,
    pub reference: &'static str,
}

/// Whether the saved account can still open the academic system. Only the
/// undergraduate adapter has a lightweight session probe today.
pub async fn academic_available(account: &crate::accounts::Account) -> bool {
    account.degree != "undergraduate" || transport::Client::connect(account.clone()).await.is_ok()
}

/// Backend availability is separate from UI completion and real-account verification.
pub fn catalog() -> Vec<Feature> {
    [
        (
            "grades",
            "成绩查询",
            "学习与教务",
            "undergraduate",
            "jwxt/score.py; gmis/score.py",
        ),
        (
            "evaluations",
            "一键评教",
            "学习与教务",
            "undergraduate",
            "jwxt/judge.py; gste/judge.py",
        ),
        (
            "exams",
            "考试安排",
            "学习与教务",
            "undergraduate",
            "jwxt/schedule.py",
        ),
        (
            "rooms",
            "空闲教室",
            "学习与教务",
            "undergraduate",
            "jwxt/empty_room.py",
        ),
        (
            "courses",
            "全校课程",
            "学习与教务",
            "undergraduate",
            "jwxt/school_course.py",
        ),
        (
            "calendar",
            "校历",
            "学习与教务",
            "public",
            "jwxt/calendar.py",
        ),
        (
            "attendance",
            "考勤查询",
            "学习与教务",
            "undergraduate",
            "attendance/attendance.py",
        ),
        (
            "lms",
            "思源学堂",
            "学习与教务",
            "undergraduate",
            "lms/lms.py",
        ),
        (
            "notices",
            "通知订阅",
            "校园生活",
            "pending",
            "app/threads/NoticeThread.py",
        ),
        (
            "card",
            "校园卡",
            "校园生活",
            "undergraduate",
            "card/campus_card.py",
        ),
        (
            "library",
            "图书馆座位",
            "校园生活",
            "undergraduate",
            "library/seats.py",
        ),
        (
            "fitness",
            "体测成绩",
            "校园生活",
            "undergraduate",
            "fitness/score.py",
        ),
        (
            "venues",
            "场馆预约",
            "校园生活",
            "undergraduate",
            "app/sessions/venue_session.py; app/venues/venue.py",
        ),
        (
            "profile",
            "学籍档案",
            "校园生活",
            "undergraduate",
            "app/sessions/hello_session.py; hello/profile.py",
        ),
    ]
    .into_iter()
    .map(|(id, name, category, backend, reference)| Feature {
        id,
        name,
        category,
        backend,
        reference,
    })
    .collect()
}
