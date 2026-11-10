//! config domain types.

export interface AppConfig {
  version: number;
  sidebarCollapsed: boolean;
  sidebarWidth: number;
  /** User order of the sidebar entries; unlisted entries go after these. */
  sidebarOrder: string[];
  /** Show the Agent entry in the sidebar; on by default. */
  agentSidebarVisible: boolean;
  /** Lets the community page's Agent search use the configured model service. */
  agentSearchEnabled: boolean;
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
  /** Plugin layout generation; 1 means the built-in plugin centre. */
  pluginLayout: number;
  checkAppUpdates: boolean;
  campusPinned: string[];
  /** 畅行校园 section rail collapsed to icons. */
  campusRailCollapsed: boolean;
  /** Clock style on the home page (plain | bold | mono | card). */
  homeClock: string;
  /** Home sections kept visible. */
  homeSections: string[];
  /** Loopback MCP endpoint on the personal page. */
  mcpServerEnabled: boolean;
  /** Loopback port chosen by the user. */
  mcpServerPort: number;
  /** Built-in tools kept out of that endpoint. */
  mcpServerDisabledTools: string[];
  /** Copy a submitted homework's files into the matching 工作台 course folder. */
  workbenchAutoArchive: boolean;
  /** Give each page-level tab its own rounded backing. */
  pageTabSurfaces: boolean;
  /** Wallpaper display preset (clear | balanced | vivid); only used with a wallpaper. */
  wallpaperDisplay: string;
  /** Panel opacity percent the user picked for each wallpaper preset. */
  wallpaperPanel: Record<string, number>;
  /** Optional mirror for the update manifest; empty uses the built-in. */
  updateEndpoint: string;
}
export interface Link {
  title: string;
  description: string;
  url: string;
}
export type PluginCategory = 'campus' | 'invention' | 'lab';
export interface Plugin {
  schemaVersion: number;
  id: string;
  name: string;
  version: string;
  category: PluginCategory;
  author: string;
  /** Licence of the plugin's own code; third-party assets live in `credits`. */
  license: string;
  credits: string;
  description: string;
  details: string;
  /** New installs start with this state; the user's switch wins afterwards. */
  enabledByDefault: boolean;
  icon: string;
  page:
    | { type: 'timetable' }
    | { type: 'links'; links: Link[] }
    | { type: 'campus' }
    | { type: 'agentSettings' }
    | { type: 'homeTheme' }
    | { type: 'pptCapture' }
    | { type: 'desktopPet' }
    | { type: 'fontBeautify' };
  widget: boolean;
  /** Sidebar entry; absent means true. Pages opened from a workflow set false. */
  sidebar?: boolean;
}
export interface PluginList {
  plugins: Plugin[];
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
