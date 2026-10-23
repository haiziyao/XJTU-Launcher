export interface AppConfig {
  version: number;
  sidebarCollapsed: boolean;
  sidebarWidth: number;
  nickname: string;
  avatar: string | null;
  wallpaper: string | null;
  reduceMotion: boolean;
  showSeconds: boolean;
  accent: string;
  pet: string;
  greeting: string;
  widgets: string[];
  pluginOrder: string[];
  disabledPlugins: string[];
  hiddenPlugins: string[];
  pluginServer: string | null;
  checkPluginUpdates: boolean;
  checkAppUpdates: boolean;
  campusPinned: string[];
  /** Clock style on the home page (plain | bold | mono | card). */
  homeClock: string;
  /** Home sections kept visible. */
  homeSections: string[];
}

export interface Link {
  title: string;
  description: string;
  url: string;
}
export interface Plugin {
  schemaVersion: number;
  id: string;
  name: string;
  version: string;
  description: string;
  icon: string;
  page:
    | { type: 'timetable' }
    | { type: 'links'; links: Link[] }
    | { type: 'campus' }
    | { type: 'agentSettings' }
    | { type: 'homeTheme' };
  widget: boolean;
}
export interface PluginList {
  plugins: Plugin[];
  warnings: string[];
  directory: string;
  sources: Record<string, string>;
}
export interface AccountStatus {
  connected: boolean;
  name?: string | null;
  studentId?: string | null;
  avatar?: string | null;
  degree: 'undergraduate' | 'postgraduate' | null;
  apiKeySet: boolean;
  accessMode?: 'direct' | 'webvpn' | null;
  /** Whether this account also holds a direct session for card/fitness/profile. */
  directSession?: boolean;
}
/** Result of the sidebar network self-check. */
export interface NetworkCheck {
  tone: 'campus' | 'vpn' | 'manual' | 'neutral';
  message: string;
}
/** Animation state of the sidebar pet. */
export type PetState = 'idle' | 'checking' | 'ok' | 'warn';
export interface Course {
  name: string;
  teacher: string;
  location: string;
  day: number;
  start: number;
  end: number;
  weeks: number[];
}
export interface Timetable {
  semester: string;
  startDate: string;
  endDate: string;
  courses: Course[];
}
export interface AuthEvent {
  state: 'success' | 'error' | 'cancelled';
  message: string;
}

export type CampusDegree = 'undergraduate' | 'postgraduate';
export interface CampusAccount {
  id: string;
  username: string;
  degree: CampusDegree;
  name: string | null;
  studentId: string | null;
  avatar: string | null;
  accessMode: 'direct' | 'webvpn';
  passwordSaved: boolean;
}
export interface CampusAccountList {
  activeId: string | null;
  accounts: CampusAccount[];
}
export interface CampusLogin {
  flowId: string;
  accessMode: 'direct' | 'webvpn';
  step: 'webvpn' | 'campus';
  phase: 'credentials' | 'captcha' | 'mfa' | 'identity' | 'success';
  phone: string | null;
  choices: { label: string; name: string }[];
  message: string | null;
  canContinue: boolean;
  retryAfterSeconds: number;
  savePending: boolean;
}
export type CampusLoginAction =
  | {
      type: 'credentials';
      username: string;
      password: string;
      captcha?: string;
      rememberPassword: boolean;
    }
  | { type: 'savedCredentials'; accountId: string; captcha?: string; rememberPassword: boolean }
  | { type: 'continue'; captcha?: string }
  | { type: 'sendMfa' }
  | { type: 'verifyMfa'; code: string }
  | { type: 'chooseIdentity'; label: string };

export type ModelProtocol = 'openai-compatible' | 'anthropic' | 'gemini';
export interface ModelProvider {
  id: string;
  name: string;
  baseUrl: string;
  protocol: ModelProtocol;
  model: string;
  /** Cached catalogue from the vendor, shown when picking a model in chat. */
  models: string[];
  keySet: boolean;
}
export interface ModelProviders {
  defaultId: string | null;
  providers: ModelProvider[];
  legacyKeyAvailable: boolean;
}
export interface ProviderInput {
  id: string | null;
  name: string;
  baseUrl: string;
  protocol: ModelProtocol;
  model: string;
  models: string[];
  apiKey: string | null;
  useLegacyKey: boolean;
}
/** Account balance for vendors that expose one; other vendors report null. */
export interface ProviderBalance {
  amount: string;
  currency: string;
}
/** Enough context to read a provider's model catalogue before saving it. */
export interface ModelListInput {
  id: string | null;
  baseUrl: string;
  protocol: ModelProtocol;
  apiKey: string | null;
  useLegacyKey: boolean;
}

export interface MarketEntry {
  id: string;
  name: string;
  description: string;
  version: string;
  minAppVersion: string;
  downloadUrl: string;
  sha256: string;
}
export interface MarketCatalog {
  updates: string[];
  source: string | null;
  defaultSource: string | null;
  plugins: MarketEntry[];
}
/** Closed query set for the campus plugin page. */
export type SchoolQuery =
  | { type: 'card' }
  | { type: 'cardTransactions'; start: string; end: string; page: number }
  | { type: 'grades' }
  | { type: 'exams' }
  | { type: 'attendanceStreams'; page: number }
  | { type: 'calendar' }
  | { type: 'roomOptions' }
  | { type: 'rooms'; campus: string; building: string; date: string; start: number; end: number }
  | { type: 'terms' }
  | { type: 'courses'; term: string | null; name: string; campus: string }
  | { type: 'evaluations'; finished: boolean }
  | { type: 'fitness' }
  | { type: 'fitnessScore'; year: string };
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
export interface SchoolFeature {
  id: string;
  name: string;
  category: string;
  backend: string;
}
export interface UpdateStatus {
  currentVersion: string;
  phase:
    | 'unconfigured'
    | 'idle'
    | 'checking'
    | 'available'
    | 'current'
    | 'downloading'
    | 'installing'
    | 'installed'
    | 'error';
  version: string | null;
  notes: string | null;
  received: number;
  total: number | null;
  message: string | null;
}
export type PersonalImageKind = 'avatar' | 'wallpaper';
/** Crop rectangle as fractions (0–1) of the source image. */
export interface ImageCrop {
  x: number;
  y: number;
  width: number;
  height: number;
}
/** MCP transport as stored in ~/.xjtu-launcher/mcp.json. */
export type McpTransport =
  | { type: 'stdio'; command: string; args?: string[]; env?: Record<string, string> }
  | { type: 'http'; url: string; headers?: Record<string, string> };
export interface McpServer {
  id: string;
  name: string;
  enabled: boolean;
  kind: 'stdio' | 'http';
  target: string;
  /** Names only; values never leave Rust. */
  secrets: string[];
  /** Keys kept, values blanked; blank values keep the stored secret when saving. */
  transport: McpTransport;
}
export interface McpTool {
  name: string;
  description: string;
  inputSchema: unknown;
}
export interface McpField {
  key: string;
  label: string;
  secret: boolean;
  placeholder: string;
}
export interface McpEntry {
  id: string;
  name: string;
  description: string;
  transport: McpTransport;
  fields: McpField[];
}
export interface McpCatalog {
  source: string | null;
  defaultSource: string | null;
  servers: McpEntry[];
}
export interface AgentToolCall {
  id: string;
  name: string;
  arguments: string;
}
export interface AgentMessage {
  role: 'user' | 'assistant' | 'tool';
  content: string;
  toolCalls?: AgentToolCall[];
  toolCallId?: string;
  name?: string;
  error?: boolean;
}
export interface AgentConversation {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  roleId: string | null;
  pinned: boolean;
  workspaceId?: string | null;
  messages: AgentMessage[];
}
export interface AgentSummary {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  pinned: boolean;
  roleId: string | null;
  workspaceId?: string | null;
  messages: number;
}
/** Named group of conversations, stored beside them. */
export interface AgentWorkspace {
  id: string;
  name: string;
  createdAt: number;
}
export interface AgentRole {
  id: string;
  name: string;
  description: string;
  instructions: string;
}
export type AgentSkill = AgentRole;
export interface AgentTask {
  id: string;
  name: string;
  description: string;
  prompt: string;
  roleId: string | null;
  skillIds: string[];
}
export interface AgentPresets {
  version: number;
  roles: AgentRole[];
  skills: AgentSkill[];
  tasks: AgentTask[];
}
export interface AgentMemory {
  id: string;
  content: string;
  createdAt: number;
}
export type AgentPresetKind = 'role' | 'skill' | 'task';
export type AgentPresetInput =
  | { kind: 'role'; id?: string | null; name: string; description: string; instructions: string }
  | { kind: 'skill'; id?: string | null; name: string; description: string; instructions: string }
  | {
      kind: 'task';
      id?: string | null;
      name: string;
      description: string;
      prompt: string;
      roleId: string | null;
      skillIds: string[];
    };
export interface AgentToolInfo {
  name: string;
  description: string;
  source: 'campus' | 'mcp';
}
export type AgentEvent =
  | { type: 'delta'; requestId: string; text: string }
  | {
      type: 'toolStart';
      requestId: string;
      callId: string;
      name: string;
      arguments: string;
    }
  | { type: 'toolEnd'; requestId: string; callId: string; ok: boolean; content: string }
  | { type: 'usage'; requestId: string; inputTokens: number; outputTokens: number }
  | { type: 'done'; requestId: string; conversationId: string }
  | { type: 'error'; requestId: string; message: string }
  | { type: 'cancelled'; requestId: string };
