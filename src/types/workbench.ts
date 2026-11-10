//! workbench domain types.

/** A course folder's hidden marker: the stable identity behind an alias. */
export interface WorkbenchCourseMeta {
  id: number;
  code: string;
  subjectCode: string;
  name: string;
  term: string;
}
/** One folder in the workspace tree; `kind` is derived from course markers. */
export interface WorkbenchFolder {
  path: string;
  name: string;
  parent: string;
  kind: 'term' | 'course' | 'folder';
  course: WorkbenchCourseMeta | null;
  /** Direct files and folders inside. */
  files: number;
  folders: number;
  modified: number;
}
export interface WorkbenchFile {
  path: string;
  name: string;
  folder: string;
  size: number;
  modified: number;
  course: WorkbenchCourseMeta | null;
  assignment: string;
  activityId: number | null;
  tags: string[];
  note: string;
}
export interface WorkbenchAssignment {
  id: number;
  title: string;
  start: string;
  end: string;
  closed: boolean;
  submitCount: number;
  submitByGroup: boolean;
}
/** One synced course: its folder plus the assignment list kept in app state. */
export interface WorkbenchCourse {
  id: number;
  code: string;
  subjectCode: string;
  name: string;
  folder: string;
  assignments: WorkbenchAssignment[];
}
export interface WorkbenchOverview {
  currentTerm: string;
  syncedAt: number;
  folders: WorkbenchFolder[];
  courses: WorkbenchCourse[];
}
export interface WorkbenchBindingInput {
  assignment: string;
  activityId: number | null;
  tags: string[];
  note: string;
}
export interface WorkbenchImportOutcome {
  path: string;
  file: WorkbenchFile | null;
  error: string | null;
}
export interface WorkbenchSyncResult {
  term: string;
  syncedAt: number;
  courses: number;
  assignments: number;
  failed: number;
}
/** What the local backend checked before any upload or post. */
export interface LmsSubmitPreview {
  confirmRequired: true;
  activity: number;
  title: string;
  courseId: number | null;
  start: string;
  end: string;
  closed: boolean;
  resubmitOpen: boolean;
  /** False when the school will refuse this submission (closed, no resubmit). */
  canSubmit: boolean;
  submitCount: number | null;
  draft: boolean;
  comment: string;
  files: { name: string; size: number }[];
  totalSize: number;
  note: string;
}
export interface LmsSubmitResult {
  activity: number;
  title: string;
  draft: boolean;
  submissionId: number | null;
  submittedAt: string;
  files: { name: string; size: number }[];
  message: string;
}
