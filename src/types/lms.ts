//! lms domain types.

export interface LmsCourse {
  id: number;
  name: string;
  code: string;
  type: string;
  credit: number | null;
  class: string;
  department: string;
  instructors: string[];
  year: string;
  term: string;
  /** Derived semester (`2026-2027 秋季学期`); empty when it cannot be worked out. */
  termLabel: string;
  start: string;
  end: string;
  published: boolean;
}
export interface LmsCourses {
  courses: LmsCourse[];
}
export interface LmsActivity {
  id: number;
  type: string;
  title: string;
  start: string;
  end: string;
  published: boolean;
  group: boolean;
}
export interface LmsActivities {
  course: number;
  activities: LmsActivity[];
}
/** One replay video for the capture page. `downloadUrl` is presigned and
 * short-lived; it must never be cached, logged or shown as text. */
export interface LmsReplayVideo {
  label: string;
  size: number;
  mute: boolean;
  bestAudio: boolean;
  downloadUrl: string;
}
export interface LmsReplayVideos {
  activity: number;
  title: string;
  videos: LmsReplayVideo[];
}
export interface LmsUpload {
  id: number;
  name: string;
  size: number | null;
  type: string;
  allowDownload: boolean;
}
export interface LmsActivityDetail {
  id: number;
  courseId: number | null;
  type: string;
  title: string;
  start: string;
  end: string;
  published: boolean;
  closed: boolean;
  resubmitOpen: boolean;
  allowRetract: boolean;
  mode: string;
  description: string;
  uploads: LmsUpload[];
  submitByGroup: boolean;
  submitCount: number | null;
  submitTimes: number | null;
  averageScore: number | null;
  highestScore: number | null;
  lowestScore: number | null;
  scoredCount: number | null;
  lessonStart: string;
  lessonEnd: string;
  duration: number | null;
  replayCode: string;
  liveName: string;
  liveStart: string;
  liveEnd: string;
  player: string;
}
export interface LmsCorrection {
  score: number | null;
  instructorScore: number | null;
  comment: string;
  updatedAt: string;
  files: LmsUpload[];
}
export interface LmsSubmission {
  id: number;
  status: string;
  score: number | null;
  rubricScore: number | null;
  submittedAt: string;
  updatedAt: string;
  createdAt: string;
  latest: boolean;
  resubmitted: boolean;
  canRetract: boolean;
  mode: string;
  comment: string;
  instructorComment: string;
  files: LmsUpload[];
  correction: LmsCorrection | null;
}
export interface LmsSubmissions {
  activity: number;
  title: string;
  courseId: number | null;
  end: string;
  closed: boolean;
  submitByGroup: boolean;
  submissions: LmsSubmission[];
}
/** Outstanding homework aggregated from the current semester's courses. */
export interface LmsTodo {
  id: number;
  courseId: number;
  course: string;
  semester: string;
  title: string;
  start: string;
  end: string;
  submitByGroup: boolean;
  submitTimes: number | null;
  submitCount: number | null;
}
/** One downloadable file from a material activity in the current semester. */
export interface LmsLibraryFile {
  courseId: number;
  course: string;
  semester: string;
  activity: number;
  title: string;
  end: string;
  file: LmsUpload;
}
export interface LmsOverview {
  semester: string;
  rows: LmsTodo[];
  files: LmsLibraryFile[];
  /** Request count that failed, so the page can say the list is partial. */
  failed: number;
}
/** One homework the school's official todo list still keeps open. */
export interface LmsTodoItem {
  id: number;
  courseId: number;
  course: string;
  courseCode: string;
  semester: string;
  title: string;
  end: string;
  /** `overdue` only means the deadline passed while the item stayed open. */
  state: 'todo' | 'overdue';
  locked: boolean;
}
/** `GET /api/todos`: the only source for 待办, one request for all courses. */
export interface LmsTodos {
  semester: string;
  items: LmsTodoItem[];
  partial: boolean;
  failures: { courseId: number; courseName: string; message: string }[];
}
