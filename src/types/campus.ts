//! campus domain types.

export interface CampusNotice {
  title: string;
  category: string;
  date: string;
  url: string;
}

export interface CampusNoticeBoard {
  channel: string;
  name: string;
  url: string;
  items: CampusNotice[];
}
/** Closed query set for the campus plugin page. */
export type SchoolQuery =
  | { type: 'card' }
  | { type: 'cardTransactions'; start: string; end: string; page: number }
  | { type: 'grades'; term?: string | null }
  | { type: 'exams'; term?: string | null }
  | { type: 'attendanceStreams'; start?: string; end?: string; page: number }
  | { type: 'attendanceRecords'; start: string; end: string; page: number }
  | { type: 'calendar' }
  | { type: 'roomOptions' }
  | { type: 'rooms'; campus: string; building: string; date: string; start: number; end: number }
  | { type: 'terms' }
  | { type: 'courses'; term: string | null; name: string; campus: string }
  | { type: 'evaluations'; finished: boolean }
  | { type: 'fitness' }
  | { type: 'fitnessScore'; year: string }
  | { type: 'fitnessItem'; item: number; year: string }
  | { type: 'profile' }
  | { type: 'lmsCourses' }
  | { type: 'lmsActivities'; course: number }
  | { type: 'lmsActivity'; activity: number }
  | { type: 'lmsSubmissions'; activity: number }
  | { type: 'lmsOverview' }
  | { type: 'lmsTodos' }
  | { type: 'noticeChannels' }
  | { type: 'notices'; channel: string; limit: number }
  | { type: 'libraryFloors' }
  | { type: 'libraryAreas'; floor: string }
  | { type: 'librarySeats'; area: string }
  | { type: 'venueList' }
  | { type: 'venueSlots'; venue: number; date: string }
  | { type: 'venueOrders' };
export interface GradeRow {
  id: string;
  name: string;
  code: string;
  term: string;
  credits: number | null;
  score: string;
  gpa: number | null;
  category: string;
  passed: boolean | null;
}
export interface Grades {
  rows: GradeRow[];
}
export interface ExamRow {
  course: string;
  date: string;
  time: string;
  location: string;
  seat: string;
  campus: string;
}
export interface Exams {
  term: string;
  rows: ExamRow[];
}
export interface AttendanceStream {
  id: string;
  place: string;
  time: string;
  valid: boolean;
}
export interface AttendanceStreams {
  page: number;
  total: number;
  rows: AttendanceStream[];
}
/** One course meeting as the attendance service reports it. */
export interface AttendanceRecord {
  id: string;
  date: string;
  start: string;
  end: string;
  week: string;
  place: string;
  teacher: string;
  /** Matched from the timetable by weekday, week and sections; empty if unknown. */
  course?: string;
  status: 'pending' | 'normal' | 'late' | 'absent' | 'leave' | 'notRequired' | 'unknown';
}
export interface AttendanceRecords {
  page: number;
  total: number;
  rows: AttendanceRecord[];
}
export interface NoticeChannel {
  id: string;
  name: string;
  level: string;
}
export interface NoticeChannels {
  channels: NoticeChannel[];
}
export interface NoticeItem {
  title: string;
  category: string;
  date: string;
  url: string;
}
export interface NoticeList {
  channel: string;
  name: string;
  url: string;
  items: NoticeItem[];
}
export interface CalendarEntry {
  year: string | null;
  url: string;
}
export interface CalendarList {
  rows: CalendarEntry[];
}
export interface Option {
  id: string;
  name: string;
}
export interface RoomOptions {
  campuses: Option[];
  buildings: Option[];
}
export interface RoomRow {
  name: string;
  building: string;
  category: string;
  capacity: number | null;
  campus: string;
}
export interface Rooms {
  rows: RoomRow[];
}
export interface Terms {
  rows: Option[];
}
export interface CourseRow {
  code: string;
  name: string;
  class: string;
  teachers: string;
  credits: number | null;
  campus: string;
  department: string;
  timePlace: string;
}
export interface Courses {
  term: string;
  rows: CourseRow[];
}
export interface Questionnaire {
  id: string;
  course: string;
  teacher: string;
  kind: string;
  term: string;
  deadline: string;
  finished: boolean;
}
export interface EvaluationList {
  rows: Questionnaire[];
}
export interface EvaluationPrepare {
  ids: string[];
  rank: number;
  comment: string;
}
export interface PreviewAnswer {
  id: string;
  question: string;
  kind: string;
  answer: string;
}
export interface PreviewCourse {
  questionnaire: Questionnaire;
  answers: PreviewAnswer[];
}
export interface EvaluationPreview {
  id: string;
  accountId: string;
  expiresInSeconds: number;
  courses: PreviewCourse[];
}
export interface FitnessYear {
  year: string;
  name: string;
  checked: boolean;
}
export interface FitnessItem {
  key: string;
  name: string;
  score: string;
  /** 换算分（平台 `*_score2`），没有时为空。 */
  converted: string;
  grade: string;
}
export interface FitnessScore {
  studentNumber: string;
  studentName: string;
  totalScore: string;
  totalGrade: string;
  reportType: string;
  reportStatus: string;
  sex: string;
  grade: string;
  items: FitnessItem[];
}
export interface FitnessYears {
  rows: FitnessYear[];
}
/**
 * One fitness item's raw measurement and history, keeping the platform shape:
 * `info.value_1` / `info.value_2` are the raw values (item 1 is height in cm
 * and weight in kg), `info.photo_url` exists for some items, `list` is the
 * history. History row fields are not part of the adapter yet.
 */
export interface FitnessItemDetail {
  info: {
    value_1?: string | number;
    value_2?: string | number;
    photo_url?: string;
  } | null;
  list: Record<string, unknown>[];
}
export interface EvaluationOutcome {
  questionnaireId: string;
  course: string;
  status: string;
  message: string;
}
export interface CardInfo {
  name: string;
  studentNo: string;
  account: string;
  balance: number;
  pending: number;
  lost: boolean;
  frozen: boolean;
  cardType: string;
  expireDate: string;
}
export interface CardTransaction {
  time: string;
  amount: number;
  balance: number;
  merchant: string;
  type: string;
  description: string;
}
export interface CardTransactions {
  page: number;
  total: number;
  rows: CardTransaction[];
}
export interface ProfileInfo {
  name: string;
  studentNo: string;
  sex: string;
  birthdate: string;
  grade: string;
  campus: string;
  academy: string;
  department: string;
  major: string;
  className: string;
  schoolingLen: string;
  enterSchoolDate: string;
  cardId: string;
  pictureUrl: string;
  classTeacher: string;
  classTeacherPhone: string;
  counselor: string;
  counselorPhone: string;
  counselorOffice: string;
}
export interface LibraryFloor {
  campus: string;
  code: string;
  name: string;
}
export interface LibraryArea {
  code: string;
  name: string;
}
export interface LibraryAreas {
  floor: string;
  areas: LibraryArea[];
}
export interface LibrarySeat {
  id: string;
  free: boolean;
}
export interface LibrarySeats {
  area: string;
  seats: LibrarySeat[];
  stats: { code: string; total: number; free: number }[];
}
export interface Venue {
  id: number;
  name: string;
  address: string;
  category: string;
  advanceDays: number;
  advanceCount: number;
}
export interface VenueList {
  venues: Venue[];
}
export interface VenueSlot {
  area: string;
  stockId: number;
  time: string;
  price: number;
  using: number;
  total: number;
  available: boolean;
}
export interface VenueSlots {
  venue: number;
  date: string;
  slots: VenueSlot[];
}
export interface VenueOrderDetail {
  date: string;
  time: string;
  area: string;
  venue: string;
}
export interface VenueOrder {
  id: string;
  status: number;
  statusText: string;
  created: string;
  price: number;
  details: VenueOrderDetail[];
}
export interface VenueOrders {
  orders: VenueOrder[];
}
export interface SchoolFeature {
  id: string;
  name: string;
  category: string;
  backend: string;
}
