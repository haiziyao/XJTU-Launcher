//! Physical-education course score, homework and student profile.
//!
//! Course scores and homework overlap with 思源学堂. The campus UI should pick
//! one source or merge them deliberately; these helpers only expose the sports
//! platform side.
use super::{data, Session};
use serde_json::Value;

const GYM_SCORE: &str = "bdlp_h5_fitness_test/public/index.php/index/course/getStudentGymScore";
const HOMEWORK: &str = "bdlp_h5_fitness_test/public/index.php/index/Coursemanage/homework_list";
const STUDENT: &str = "bdlp_h5_fitness_test/public/index.php/index/User/getStudentInfo";

pub(crate) struct Client<'a> {
    pub(super) session: &'a Session,
}

impl Client<'_> {
    /// Physical-education course score.
    pub(crate) async fn gym_score(&self) -> Result<Value, String> {
        let value = self.session.post(GYM_SCORE, &[]).await?;
        Ok(data(&value))
    }

    /// Homework list for one course.
    pub(crate) async fn homework(&self, course_id: &str) -> Result<Value, String> {
        let value = self
            .session
            .post(HOMEWORK, &[("act", "get"), ("course_id", course_id)])
            .await?;
        Ok(data(&value))
    }

    /// Student profile on the sports platform (name, number, photo URL).
    pub(crate) async fn student(&self) -> Result<Value, String> {
        let value = self.session.post(STUDENT, &[("type", "1")]).await?;
        Ok(data(&value))
    }
}
