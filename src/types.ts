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
  greeting: string;
  widgets: string[];
  pluginOrder: string[];
  disabledPlugins: string[];
  hiddenPlugins: string[];
  pluginServer: string | null;
  checkPluginUpdates: boolean;
  checkAppUpdates: boolean;
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
  page: { type: 'timetable' } | { type: 'links'; links: Link[] };
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
}
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
