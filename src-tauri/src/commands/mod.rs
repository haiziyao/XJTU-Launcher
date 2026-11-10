//! Command layer grouped by feature domain.

pub(crate) mod accounts;
pub(crate) mod agent;
pub(crate) mod config;
pub(crate) mod expose;
pub(crate) mod lms;
pub(crate) mod mcp;
pub(crate) mod network;
pub(crate) mod personal;
pub(crate) mod providers;
pub(crate) mod school;
pub(crate) mod session;
pub(crate) mod storage;
pub(crate) mod system;
pub(crate) mod timetable;
pub(crate) mod workbench;

// These helpers are also used by agent tools, cache and other core modules, so
// they stay reachable as `crate::<name>` even though their commands live under
// `commands::<domain>`.
pub(crate) use providers::providers_path;
pub(crate) use school::{
    download_lms_file, prepare_evaluations, run_lms_homework, run_school_query, submit_evaluations,
};
pub(crate) use timetable::{
    apply_timetable_edit, merge_cached_timetable, run_timetable, timetable_customizations,
};
pub(crate) use workbench::{run_workbench_edit, run_workbench_read};
