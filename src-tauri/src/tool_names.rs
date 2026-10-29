//! Campus tool names that were merged in 0.5.2.
//!
//! Kept in its own module because `config.rs` is compiled standalone by the
//! smoke-test examples, which do not include the Agent modules.

/// Pre-merge tool name, the merged tool that now owns it, and the action the
/// old call implies. `None` when the name did not change.
pub fn merged(name: &str) -> Option<(&'static str, Option<&'static str>)> {
    Some(match name {
        "campus_timetable_customizations" => ("campus_timetable", Some("customizations")),
        "campus_terms" => ("campus_academic", Some("terms")),
        "campus_grades" => ("campus_academic", Some("grades")),
        "campus_exams" => ("campus_academic", Some("exams")),
        "campus_calendar" => ("campus_academic", Some("calendar")),
        "campus_profile" => ("campus_academic", Some("profile")),
        "campus_courses" => ("campus_academic", Some("courses")),
        "campus_attendance_streams" => ("campus_attendance", Some("streams")),
        "campus_attendance_records" => ("campus_attendance", Some("records")),
        "campus_card_transactions" => ("campus_card", Some("transactions")),
        "campus_fitness_score" => ("campus_fitness", Some("score")),
        "campus_room_options" => ("campus_classroom", Some("options")),
        "campus_rooms" => ("campus_classroom", Some("rooms")),
        "campus_evaluations" => ("campus_evaluation", Some("list")),
        "campus_evaluations_preview" => ("campus_evaluation", Some("preview")),
        "campus_notice_channels" => ("campus_notices", Some("channels")),
        "campus_lms_courses" => ("campus_lms", Some("courses")),
        "campus_lms_activities" => ("campus_lms", Some("activities")),
        "campus_lms_activity" => ("campus_lms", Some("activity")),
        "campus_lms_submissions" => ("campus_lms", Some("submissions")),
        "campus_library_floors" => ("campus_library", Some("floors")),
        "campus_library_areas" => ("campus_library", Some("areas")),
        "campus_library_seats" => ("campus_library", Some("seats")),
        "campus_venue_slots" => ("campus_venues", Some("slots")),
        "campus_venue_orders" => ("campus_venues", Some("orders")),
        _ => return None,
    })
}

/// Canonical tool name for the per-tool exposure setting. A name from before
/// the merge maps to its merged tool, which hides the whole domain: hiding
/// more than the user asked is safer than exposing a tool they disabled.
pub fn canonical_name(name: &str) -> &str {
    merged(name).map(|(tool, _)| tool).unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_old_names_to_merged_tools() {
        assert_eq!(canonical_name("campus_card_transactions"), "campus_card");
        assert_eq!(
            canonical_name("campus_evaluations_preview"),
            "campus_evaluation"
        );
        assert_eq!(canonical_name("campus_card"), "campus_card");
        let (tool, action) = merged("campus_grades").unwrap();
        assert_eq!(tool, "campus_academic");
        assert_eq!(action, Some("grades"));
    }
}
