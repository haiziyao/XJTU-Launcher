//! timetable domain types.

export interface Course {
  /** Stable within one semester; absent on the school's own raw list. */
  id?: string;
  name: string;
  teacher: string;
  location: string;
  day: number;
  start: number;
  end: number;
  weeks: number[];
  /** Hue 0–5 picked by the user; absent means the name picks the colour. */
  color?: number;
  /** Added on this machine rather than fetched from the school. */
  custom?: boolean;
  /** A school course whose fields come from a local override. */
  edited?: boolean;
}
export interface Timetable {
  semester: string;
  startDate: string;
  endDate: string;
  courses: Course[];
  /** School courses hidden locally; the page offers a way to bring them back. */
  hidden?: Course[];
}
/** Fields of a course the timetable editor can set. */
export interface TimetableCourse {
  name: string;
  teacher: string;
  location: string;
  day: number;
  start: number;
  end: number;
  weeks: number[] | string;
  color?: number;
}
/** One local timetable edit, as `edit_timetable` takes it. */
export interface TimetableEdit {
  semester: string;
  action: 'save' | 'hide' | 'reset';
  id?: string;
  course?: TimetableCourse;
}
