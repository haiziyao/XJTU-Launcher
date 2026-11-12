<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import type { Component } from 'vue';
import {
  AlertCircle,
  ArrowLeft,
  Bot,
  ChevronLeft,
  Contrast,
  Download,
  Home,
  LayoutGrid,
  LoaderCircle,
  MessagesSquare,
  Briefcase,
  Settings,
  Wifi,
  WifiOff,
  X,
} from 'lucide-vue-next';
import { initializeUpdates, updateStatus } from './shared/updates';
import { bridge, native } from './shared/bridge';
import { ensureDirectSession } from './features/campus/campusSession';
import { prefetchCampus } from './features/campus/campusPrefetch';
import { resetLmsCaches } from './features/campus/campusOverview';
import {
  coursesForDay,
  defaultConfig,
  mergeSidebarOrder,
  orderedPlugins,
  orderSidebarEntries,
} from './shared/domain';
import { pluginGlyph } from './features/plugins/pluginGlyph';
import type {
  AccountStatus,
  AppConfig,
  CampusNoticeBoard,
  FontPluginSettings,
  NetworkCheck,
  PetState,
  PluginList,
  Timetable,
} from './types';
import SidebarPet from './components/SidebarPet.vue';
import { normalizePet } from './shared/pets';
import HomeView from './features/home/HomeView.vue';
import AccountsView from './features/accounts/AccountsView.vue';
import PersonalView from './features/personal/PersonalView.vue';
import SettingsView from './features/settings/SettingsView.vue';
import CommunityView from './features/community/CommunityView.vue';
import WorkbenchView from './features/workbench/WorkbenchView.vue';
import AgentView from './features/agent/AgentView.vue';
import AgentSettingsView from './features/agent/AgentSettingsView.vue';
import HomeThemeView from './features/appearance/HomeThemeView.vue';
import UpdateDialog from './components/UpdateDialog.vue';
import { useSidebarResize } from './composables/useSidebarResize';
import PluginMarketView from './features/plugins/PluginMarketView.vue';
import LinksView from './features/home/LinksView.vue';
import TimetableView from './features/campus/TimetableView.vue';
import { applyCached } from './shared/cache';
import { loadEntryIcons } from './shared/icons';
import CampusView from './features/campus/CampusView.vue';
import PptCaptureView from './features/plugins/PptCaptureView.vue';
import DesktopPetView from './features/appearance/DesktopPetView.vue';
import FontBeautifyView from './features/appearance/FontBeautifyView.vue';
import type { CaptureTarget } from './features/plugins/capture/target';
import EmptyState from './components/ui/EmptyState.vue';
import EntryIcon from './components/ui/EntryIcon.vue';
import UiAvatar from './components/ui/UiAvatar.vue';
import UiButton from './components/ui/UiButton.vue';
import UiIconButton from './components/ui/UiIconButton.vue';
import { useSlidingIndicator } from './composables/useSlidingIndicator';
import {
  DEFAULT_FONT_SETTINGS,
  FONT_PLUGIN_ID,
  applyFonts,
  normalizeFontSettings,
} from './features/appearance/fonts';
import {
  WALLPAPER_MODES,
  applyAccent,
  normalizeAccent,
  wallpaperMode,
  wallpaperPanelOpacity,
  wallpaperTokens,
} from './features/appearance/theme';
import './styles/ui.css';
import './styles/pages.css';
import './styles/fonts.css';
import './styles/campus.css';
import './styles/campus-v2.css';
import './styles/desktop-shell.css';
import './styles/pet-toast.css';
import './styles/pet-art.css';

const route = ref('home');
/** Set when 思源学堂 hands a lesson to the capture page; that page reads it on mount. */
const captureTarget = ref<CaptureTarget | null>(null);
function openCapture(target: CaptureTarget) {
  const plugin = enabledPlugins.value.find((p) => p.page.type === 'pptCapture');
  if (!plugin) {
    showToast('回放截取已停用，先在「插件中心」里启用', 'neutral');
    return;
  }
  captureTarget.value = target;
  route.value = plugin.id;
}
const navigation = ref<HTMLElement | null>(null);
const content = ref<HTMLElement | null>(null);
const navIndicator = useSlidingIndicator(navigation, () => route.value);
// Start each page at the top once the previous one has faded out.
function resetScroll() {
  content.value?.scrollTo({ top: 0 });
}
const settingsSection = ref<'appearance' | 'home' | 'updates' | 'storage'>('appearance');
function openSettings(section: 'appearance' | 'home' | 'updates' | 'storage' = 'appearance') {
  settingsSection.value = section;
  route.value = 'settings';
  // The icon file is hand-edited; picking it up here means no restart.
  if (native) void loadEntryIcons();
}
const config = ref(defaultConfig());
const configLoaded = ref(false);
const petVariant = computed(() => normalizePet(config.value.pet));
// Accent tokens live on the document root so teleported dialogs inherit them too.
watch(() => config.value.accent, applyAccent, { immediate: true });
// The font plugin only changes the document when it is enabled; its settings
// are still loaded while it is off so turning it on applies them at once.
const fontSettings = ref<FontPluginSettings>(DEFAULT_FONT_SETTINGS);
const fontPluginEnabled = computed(() => !config.value.disabledPlugins.includes(FONT_PLUGIN_ID));
watch(
  [fontSettings, fontPluginEnabled],
  ([settings, enabled]) => void applyFonts(settings, enabled),
  {
    deep: true,
    immediate: true,
  },
);
async function reloadFontSettings() {
  try {
    fontSettings.value = normalizeFontSettings(
      await bridge.pluginSettings<FontPluginSettings>(FONT_PLUGIN_ID),
    );
  } catch {
    // A damaged plugin file falls back to the defaults in `applyFonts`.
  }
}
// Hiding the entry must not leave the app on a page with no way back.
watch(
  () => config.value.agentSidebarVisible,
  (visible) => {
    if (!visible && route.value === 'agent') route.value = 'home';
  },
  { immediate: true },
);
const pluginList = ref<PluginList>({ plugins: [] });
const account = ref<AccountStatus>({ connected: false, degree: null, apiKeySet: false });
const timetable = ref<Timetable | null>(null);
const notices = ref<CampusNoticeBoard | null>(null);
const updateDialogOpen = ref(false);
let promptedUpdate = '';
const noticesLoading = ref(false);
const noticesError = ref('');
const scheduleError = ref('');
const scheduleLoading = ref(false);
const ready = ref(false);
const error = ref('');
const saving = ref(false);
const now = ref(new Date());
const petState = ref<PetState>('idle');
const petMessage = ref('');
const toast = ref<{ id: number; message: string; tone: ToastTone } | null>(null);
let petTimer: ReturnType<typeof setTimeout> | undefined;
let toastTimer: ReturnType<typeof setTimeout> | undefined;
let toastId = 0;
type ToastTone = NetworkCheck['tone'];
function showToast(message: string, tone: ToastTone) {
  if (toastTimer) clearTimeout(toastTimer);
  toastId += 1;
  toast.value = { id: toastId, message, tone };
  // The network result is a glance-only message; it clears itself.
  toastTimer = setTimeout(() => (toast.value = null), 3000);
}
const macNative = native && /Mac/.test(navigator.userAgent);
const windowsNative = native && /Windows/.test(navigator.userAgent);
// The embedded macOS title bar paints the window buttons over the rail's top-left.
const sidebarCollapsedWidth = macNative ? 76 : 56;
const sortedPlugins = computed(() => orderedPlugins(pluginList.value.plugins, config.value));
const enabledPlugins = computed(() =>
  sortedPlugins.value.filter((p) => !config.value.disabledPlugins.includes(p.id)),
);
// The sidebar is one draggable list above the fixed footer. Plugins sit in the
// same list and can move anywhere. The old 插件 heading is gone; a plain
// divider after the last main entry still splits the rail into two levels.
interface SidebarEntry {
  id: string;
  title: string;
  iconName: string;
  icon: Component;
  visible: boolean;
  primary: boolean;
}
type SidebarItem =
  { kind: 'entry'; id: string; entry: SidebarEntry } | { kind: 'divider'; id: string };
// The divider is stored in the order as a marker, so it stays put while
// entries cross it instead of chasing the last main entry.
const SIDEBAR_DIVIDER = 'nav.divider';

const sidebarCandidates = computed<SidebarEntry[]>(() => {
  const entries: SidebarEntry[] = [
    { id: 'home', title: '主页', iconName: 'nav.home', icon: Home, visible: true, primary: true },
    {
      id: 'agent',
      title: 'Agent',
      iconName: 'nav.agent',
      icon: Bot,
      visible: config.value.agentSidebarVisible,
      primary: true,
    },
    {
      id: 'community',
      title: '社区广场',
      iconName: 'nav.community',
      icon: MessagesSquare,
      visible: true,
      primary: true,
    },
  ];
  for (const plugin of sortedPlugins.value) {
    if (plugin.page.type !== 'campus') continue;
    entries.push({
      id: plugin.id,
      title: plugin.name,
      iconName: `plugin.${plugin.id}`,
      icon: pluginGlyph(plugin),
      visible: !config.value.disabledPlugins.includes(plugin.id),
      primary: true,
    });
  }
  entries.push({
    id: 'workbench',
    title: '工作台',
    iconName: 'nav.workbench',
    icon: Briefcase,
    visible: true,
    primary: true,
  });
  for (const plugin of sortedPlugins.value) {
    if (plugin.sidebar === false || plugin.page.type === 'campus') continue;
    entries.push({
      id: plugin.id,
      title: plugin.name,
      iconName: `plugin.${plugin.id}`,
      icon: pluginGlyph(plugin),
      visible: !config.value.disabledPlugins.includes(plugin.id),
      primary: false,
    });
  }
  return orderSidebarEntries(
    entries,
    config.value.sidebarOrder.filter((id) => id !== SIDEBAR_DIVIDER),
  );
});

// The footer is deliberately fixed: it is not part of the draggable order.
const footerEntries = computed<SidebarEntry[]>(() => [
  {
    id: 'update',
    title: '发现新版本',
    iconName: 'nav.update',
    icon: Download,
    visible: updateReady.value,
    primary: false,
  },
  {
    id: 'market',
    title: '插件中心',
    iconName: 'nav.market',
    icon: LayoutGrid,
    visible: true,
    primary: false,
  },
  {
    id: 'settings',
    title: '应用设置',
    iconName: 'nav.settings',
    icon: Settings,
    visible: true,
    primary: false,
  },
]);
const footerItems = computed(() => footerEntries.value.filter((entry) => entry.visible));

// Drag state. The whole list above the footer moves as one; hidden entries
// keep their slots and the result is written back in one pass.
const draggedEntry = ref('');
const dragOrder = ref<string[] | null>(null);
/** Entries above the divider while a drag is in progress. */
const dragDividerAt = ref<number | null>(null);
let dragPointer = 0;
let dragStartY = 0;
let dragId = '';
let suppressNavClick = false;
let dragGhost: HTMLElement | null = null;
let dragGhostOffsetY = 0;

const sidebarEntries = computed(() => {
  const base = sidebarCandidates.value.filter((entry) => entry.visible);
  if (!dragOrder.value) return base;
  const byId = new Map(base.map((entry) => [entry.id, entry]));
  return dragOrder.value
    .map((id) => byId.get(id))
    .filter((entry): entry is SidebarEntry => entry !== undefined);
});

/**
 * How many entries sit above the divider. A stored marker keeps the divider
 * anchored where it is; until the first drag it follows the last main entry.
 */
const dividerSlot = computed(() => {
  if (dragDividerAt.value !== null) return dragDividerAt.value;
  const entries = sidebarEntries.value;
  if (!entries.some((entry) => !entry.primary)) return -1;
  const order = config.value.sidebarOrder;
  const marker = order.indexOf(SIDEBAR_DIVIDER);
  if (marker === -1) {
    const lastPrimary = [...entries].reverse().find((entry) => entry.primary);
    return lastPrimary ? entries.indexOf(lastPrimary) + 1 : 0;
  }
  const visible = new Set(entries.map((entry) => entry.id));
  return order.slice(0, marker).filter((id) => visible.has(id)).length;
});

const sidebarItems = computed<SidebarItem[]>(() => {
  const entries = sidebarEntries.value;
  const slot = dividerSlot.value;
  if (slot < 0) return entries.map((entry) => ({ kind: 'entry', id: entry.id, entry }));
  const items: SidebarItem[] = entries.map((entry) => ({
    kind: 'entry',
    id: entry.id,
    entry,
  }));
  items.splice(Math.max(0, Math.min(slot, entries.length)), 0, {
    kind: 'divider',
    id: SIDEBAR_DIVIDER,
  });
  return items;
});

/** A floating copy follows the pointer; the real row stays as a faded slot. */
function liftSidebarItem(event: PointerEvent) {
  const origin = navigation.value?.querySelector<HTMLElement>(
    `[data-nav-id="${CSS.escape(dragId)}"]`,
  );
  if (!origin) return;
  const rect = origin.getBoundingClientRect();
  const ghost = origin.cloneNode(true) as HTMLElement;
  ghost.classList.add('nav-drag-ghost');
  ghost.classList.remove('selected', 'dragging');
  ghost.removeAttribute('data-indicator');
  ghost.removeAttribute('aria-current');
  ghost.style.width = `${rect.width}px`;
  ghost.style.height = `${rect.height}px`;
  ghost.style.left = `${rect.left}px`;
  ghost.style.top = `${rect.top}px`;
  dragGhostOffsetY = event.clientY - rect.top;
  document.body.appendChild(ghost);
  dragGhost = ghost;
}

function dropSidebarGhost() {
  dragGhost?.remove();
  dragGhost = null;
}

function saveSidebarOrder() {
  const visible = sidebarEntries.value.map((entry) => entry.id);
  const full = mergeSidebarOrder(
    sidebarCandidates.value.map((entry) => entry.id),
    visible,
  );
  const slot = dividerSlot.value;
  if (slot >= 0) {
    const anchor = slot > 0 ? visible[slot - 1] : '';
    const index = anchor ? full.indexOf(anchor) + 1 : 0;
    full.splice(index, 0, SIDEBAR_DIVIDER);
  }
  void persist({ ...config.value, sidebarOrder: full });
}

function startSidebarDrag(event: PointerEvent, entry: SidebarEntry) {
  if (event.button !== 0) return;
  dragPointer = event.pointerId;
  dragStartY = event.clientY;
  dragId = entry.id;
  draggedEntry.value = '';
  window.addEventListener('pointermove', moveSidebarDrag);
  window.addEventListener('pointerup', endSidebarDrag);
  window.addEventListener('pointercancel', endSidebarDrag);
}

function moveSidebarDrag(event: PointerEvent) {
  if (event.pointerId !== dragPointer) return;
  if (!draggedEntry.value) {
    if (Math.abs(event.clientY - dragStartY) < 4) return;
    dragDividerAt.value = dividerSlot.value;
    dragOrder.value = sidebarEntries.value.map((entry) => entry.id);
    draggedEntry.value = dragId;
    liftSidebarItem(event);
    event.preventDefault();
  }
  if (dragGhost) dragGhost.style.top = `${event.clientY - dragGhostOffsetY}px`;
  const order = dragOrder.value;
  const buttons = navigation.value?.querySelectorAll<HTMLElement>('[data-nav-id]');
  if (!order || !buttons?.length) return;
  let target = 0;
  buttons.forEach((button, index) => {
    const rect = button.getBoundingClientRect();
    if (event.clientY > rect.top + rect.height / 2) target = index + 1;
  });
  const from = order.indexOf(dragId);
  if (from === -1) return;
  if (from < target) target -= 1;
  target = Math.max(0, Math.min(target, order.length - 1));
  const slot = dragDividerAt.value;
  if (slot !== null && slot >= 0) {
    // Which side of the line the pointer is on decides the drop side, so the
    // area right under the divider is usable instead of a dead zone.
    const base = from < slot ? slot - 1 : slot;
    const divider = navigation.value?.querySelector<HTMLElement>('.nav-divider');
    const above = divider
      ? event.clientY < divider.getBoundingClientRect().top + divider.offsetHeight / 2
      : from < slot;
    target = above ? Math.min(target, base) : Math.max(target, base);
    target = Math.max(0, Math.min(target, order.length - 1));
    dragDividerAt.value = above ? base + 1 : base;
  }
  if (target === from) return;
  const next = [...order];
  next.splice(target, 0, ...next.splice(from, 1));
  dragOrder.value = next;
}

function endSidebarDrag(event: PointerEvent) {
  if (event.pointerId !== dragPointer) return;
  window.removeEventListener('pointermove', moveSidebarDrag);
  window.removeEventListener('pointerup', endSidebarDrag);
  window.removeEventListener('pointercancel', endSidebarDrag);
  const order = dragOrder.value;
  if (order) {
    saveSidebarOrder();
    // A drag ends with a click on the item it started from; ignore that one.
    suppressNavClick = true;
    setTimeout(() => (suppressNavClick = false), 0);
  }
  dropSidebarGhost();
  draggedEntry.value = '';
  dragOrder.value = null;
  dragDividerAt.value = null;
}

function activateNav(id: string) {
  if (suppressNavClick) {
    suppressNavClick = false;
    return;
  }
  if (id === 'update') {
    updateDialogOpen.value = true;
    return;
  }
  if (id === 'settings') {
    openSettings();
    return;
  }
  route.value = id;
}
const activePlugin = computed(() => enabledPlugins.value.find((p) => p.id === route.value));
// The campus workspace holds per-account data (including the local 资料库).
// Remount it when the active account changes so a switch never leaves the
// previous account's files or school rows on screen.
const campusScope = computed(() =>
  [
    activePlugin.value?.id,
    account.value.studentId ?? account.value.name ?? '',
    account.value.degree ?? '',
  ].join(':'),
);
const showWallpaper = computed(() => !!config.value.wallpaper);
// A wallpaper preset moves the veil, panels and controls together; its text
// tokens are derived from the panel's worst-case backdrop, not plain white.
const wallpaperPreset = computed(() => wallpaperMode(config.value.wallpaperDisplay));
const wallpaperPanel = computed(() =>
  wallpaperPanelOpacity(config.value.wallpaperPanel, wallpaperPreset.value),
);
const wallpaperStyle = computed<Record<string, string>>(() =>
  showWallpaper.value
    ? wallpaperTokens(wallpaperPreset.value, wallpaperPanel.value, config.value.accent)
    : {},
);
function cycleWallpaper() {
  const index = WALLPAPER_MODES.findIndex((mode) => mode.id === wallpaperPreset.value.id);
  const next = WALLPAPER_MODES[(index + 1) % WALLPAPER_MODES.length];
  void persist({ ...config.value, wallpaperDisplay: next.id });
}
const updateReady = computed(() =>
  ['available', 'downloading', 'installing', 'installed'].includes(updateStatus.value?.phase ?? ''),
);
// Open the release dialog once per version when the launch check finds one.
watch(
  () => [updateStatus.value?.phase, updateStatus.value?.version] as const,
  ([phase, version]) => {
    if (phase === 'available' && version && promptedUpdate !== version) {
      promptedUpdate = version;
      updateDialogOpen.value = true;
    }
  },
  { immediate: true },
);
watch(enabledPlugins, () => {
  if (
    ![
      'home',
      'settings',
      'market',
      'personal',
      'accounts',
      'community',
      'agent',
      'workbench',
    ].includes(route.value) &&
    !activePlugin.value
  )
    route.value = 'home';
});
watch(route, () => {
  if (activePlugin.value?.page.type === 'timetable' && !timetable.value) void refreshTimetable();
});
async function reloadAccount() {
  const generation = ++accountGeneration;
  timetable.value = null;
  scheduleError.value = '';
  resetLmsCaches();
  try {
    const next = await bridge.account();
    if (!disposed && generation === accountGeneration) account.value = next;
  } catch (e) {
    error.value = errorText(e);
  }
}
/** A direct session was added; refresh flags without dropping the timetable. */
async function refreshAccountStatus() {
  const generation = ++accountGeneration;
  try {
    const next = await bridge.account();
    if (!disposed && generation === accountGeneration) account.value = next;
  } catch (e) {
    error.value = errorText(e);
  }
}
/**
 * Prepare the extra direct CAS session (card, fitness) as soon as an account is
 * ready, rather than waiting for the first direct-only tool. Failures stay
 * silent: opening the tool retries and explains the reason.
 */
async function warmDirectSession(status: AccountStatus) {
  if (status.accessMode !== 'webvpn' || status.directSession) return;
  const generation = accountGeneration;
  try {
    await ensureDirectSession();
  } catch {
    return;
  }
  if (disposed || generation !== accountGeneration) return;
  account.value = { ...account.value, directSession: true };
}
const timetableEnabled = computed(() =>
  sortedPlugins.value.some(
    (p) => p.page.type === 'timetable' && !config.value.disabledPlugins.includes(p.id),
  ),
);
const todayCourses = computed(() =>
  timetable.value ? coursesForDay(timetable.value, now.value) : [],
);
const displayName = computed(
  () =>
    config.value.nickname ||
    account.value.name ||
    (account.value.connected ? '校园账号' : '未登录'),
);
const displayAvatar = computed(() => config.value.avatar || account.value.avatar);
const identity = computed(() =>
  [displayName.value, account.value.studentId].filter(Boolean).join(' · '),
);
const errorText = (e: unknown) => (e instanceof Error ? e.message : String(e));
let unlisten: (() => void) | undefined;
const petUnlisteners: (() => void)[] = [];
let timer: ReturnType<typeof setInterval> | undefined;
let accountGeneration = 0;
let disposed = false;

let pendingConfig: AppConfig | null = null;
async function persist(next: AppConfig) {
  if (!configLoaded.value) return;
  const previous = config.value;
  config.value = next;
  pendingConfig = JSON.parse(JSON.stringify(next)) as AppConfig;
  if (saving.value) return;
  saving.value = true;
  let lastSaved = previous;
  try {
    while (pendingConfig) {
      const snapshot = pendingConfig;
      pendingConfig = null;
      try {
        await bridge.saveConfig(snapshot);
        lastSaved = snapshot;
      } catch (e) {
        error.value = errorText(e);
        if (!pendingConfig) config.value = lastSaved;
      }
    }
  } finally {
    saving.value = false;
  }
}
const {
  width: sidebarWidth,
  collapsed: sidebarCollapsed,
  collapsedWidth,
  resizing,
  ...resize
} = useSidebarResize(config, configLoaded, persist, sidebarCollapsedWidth);
function toggleSidebar() {
  resize.cancel();
  void persist({ ...config.value, sidebarCollapsed: !config.value.sidebarCollapsed });
}
function onKey(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key === '\\') {
    event.preventDefault();
    toggleSidebar();
  }
}
// Clicking the sidebar pet checks whether the campus host answers directly and,
// if not, through WebVPN, then reports which network the app is on.
async function checkNetwork() {
  if (petState.value === 'checking') return;
  if (petTimer) clearTimeout(petTimer);
  if (toastTimer) clearTimeout(toastTimer);
  toast.value = null;
  petState.value = 'checking';
  petMessage.value = '';
  const started = performance.now();
  let next: PetState = 'warn';
  let tone: ToastTone = 'neutral';
  let message = '';
  let directSession = false;
  try {
    const result = await bridge.checkNetwork();
    message = result.message;
    tone = result.tone;
    directSession = result.directSession;
    next = result.tone === 'campus' || result.tone === 'vpn' ? 'ok' : 'warn';
  } catch (e) {
    message = errorText(e);
  }
  // The self-check may have prepared the direct session in the background; pick
  // that up so card/fitness stop showing "需直连" without opening them first.
  if (account.value.accessMode === 'webvpn' && directSession && !account.value.directSession) {
    account.value = { ...account.value, directSession: true };
  }
  // A failed launch-time timetable fetch may have run before the self-check
  // renewed the campus session; try it again now that the check passed.
  if (scheduleError.value && next === 'ok') void refreshTimetable();
  // Keep the thinking pose on screen long enough to read.
  await new Promise((resolve) =>
    setTimeout(resolve, Math.max(0, 900 - (performance.now() - started))),
  );
  if (disposed) return;
  petMessage.value = message;
  petState.value = next;
  showToast(message, tone);
  petTimer = setTimeout(() => {
    petState.value = 'idle';
    petMessage.value = '';
  }, 3000);
}
async function refreshNotices() {
  if (!native || noticesLoading.value) return;
  noticesLoading.value = true;
  noticesError.value = '';
  try {
    const board = await bridge.notices('dean', 20);
    if (!disposed) notices.value = board;
  } catch (e) {
    if (!disposed) noticesError.value = errorText(e);
  } finally {
    if (!disposed) noticesLoading.value = false;
  }
}

async function openLink(url: string) {
  try {
    await bridge.openUrl(url);
  } catch (e) {
    error.value = errorText(e);
  }
}
async function togglePlugin(id: string) {
  const disabled = config.value.disabledPlugins.includes(id);
  await persist({
    ...config.value,
    disabledPlugins: disabled
      ? config.value.disabledPlugins.filter((p) => p !== id)
      : [...config.value.disabledPlugins, id],
  });
  if (!timetableEnabled.value) {
    timetable.value = null;
    scheduleError.value = '';
  }
}
async function refreshTimetable() {
  if (scheduleLoading.value || !timetableEnabled.value || !account.value.connected) return;
  const generation = accountGeneration;
  scheduleLoading.value = true;
  scheduleError.value = '';
  try {
    const result = await bridge.timetable('cn.xjtu.timetable');
    if (!disposed && generation === accountGeneration && timetableEnabled.value) {
      timetable.value = result;
    }
  } catch (e) {
    if (!disposed && generation === accountGeneration) scheduleError.value = errorText(e);
  } finally {
    scheduleLoading.value = false;
  }
}
/** Show the stored timetable first, then refresh it in the background. */
async function primeTimetable() {
  if (!timetableEnabled.value || !account.value.connected) return;
  const generation = accountGeneration;
  await applyCached<Timetable>('timetable', undefined, (value) => {
    if (!disposed && generation === accountGeneration && timetableEnabled.value && !timetable.value)
      timetable.value = value;
  });
  if (!disposed && generation === accountGeneration) void refreshTimetable();
}
onMounted(async () => {
  if (!native) return;
  void loadEntryIcons();
  window.addEventListener('keydown', onKey);
  timer = setInterval(() => {
    now.value = new Date();
  }, 1000);
  const results = await Promise.allSettled([bridge.config(), bridge.plugins(), bridge.account()]);
  if (disposed) return;
  if (results[0].status === 'fulfilled') {
    config.value = {
      ...results[0].value,
      accent: normalizeAccent(results[0].value.accent),
      pet: normalizePet(results[0].value.pet),
    };
    configLoaded.value = true;
  } else error.value = errorText(results[0].reason);
  if (results[1].status === 'fulfilled') pluginList.value = results[1].value;
  else error.value = errorText(results[1].reason);
  if (results[2].status === 'fulfilled') {
    account.value = results[2].value;
    void warmDirectSession(results[2].value);
    void prefetchCampus(results[2].value);
  } else error.value = errorText(results[2].reason);
  try {
    const stop = await bridge.onAuth(async (event) => {
      if (disposed) return;
      if (event.state === 'success') {
        const generation = ++accountGeneration;
        timetable.value = null;
        scheduleError.value = '';
        resetLmsCaches();
        let next: AccountStatus | undefined;
        try {
          next = await bridge.account();
          if (!disposed && generation === accountGeneration) account.value = next;
        } catch (e) {
          error.value = errorText(e);
        }
        if (!disposed && generation === accountGeneration) void primeTimetable();
        if (next && !disposed && generation === accountGeneration) {
          void warmDirectSession(next);
          void prefetchCampus(next, true);
        }
      } else if (event.state === 'error') error.value = event.message;
    });
    if (disposed) stop();
    else unlisten = stop;
  } catch (e) {
    error.value = errorText(e);
  }
  try {
    const stops = await Promise.all([
      bridge.onPetOpenSettings(() => {
        if (disposed) return;
        const pet = enabledPlugins.value.find((plugin) => plugin.page.type === 'desktopPet');
        route.value = pet ? pet.id : 'market';
      }),
    ]);
    if (disposed) stops.forEach((stop) => stop());
    else petUnlisteners.push(...stops);
  } catch (e) {
    error.value = errorText(e);
  }
  ready.value = true;
  // Home can show today's courses right away from the stored timetable.
  void primeTimetable();
  // Public notice board on the home page; independent of the campus account.
  void refreshNotices();
  // Run the same self-check as the sidebar pet once per launch.
  setTimeout(() => void checkNetwork(), 1500);
  void initializeUpdates(config.value.checkAppUpdates).catch((e) => (error.value = errorText(e)));
  void reloadFontSettings();
});
onBeforeUnmount(() => {
  disposed = true;
  unlisten?.();
  petUnlisteners.forEach((stop) => stop());
  if (timer) clearInterval(timer);
  if (petTimer) clearTimeout(petTimer);
  if (toastTimer) clearTimeout(toastTimer);
  window.removeEventListener('keydown', onKey);
});
</script>

<template>
  <div v-if="!native" class="desktop-required">请从 XJTU Launcher 桌面应用打开。</div>
  <div
    v-else
    class="desktop-shell"
    :class="{
      'sidebar-collapsed': sidebarCollapsed,
      'sidebar-resizing': resizing,
      'mac-native': macNative,
      'platform-windows': windowsNative,
      'has-wallpaper': showWallpaper,
      'tab-surfaces': config.pageTabSurfaces,
      'motion-ready': ready,
      'reduce-motion': config.reduceMotion,
    }"
    :style="{
      '--sidebar-width': sidebarWidth + 'px',
      '--sidebar-collapsed-width': collapsedWidth + 'px',
      ...wallpaperStyle,
    }"
  >
    <!-- Spans the whole window so sidebar width changes never rescale the image.
         An <img> src is patched only when it changes; a style binding would re-apply the
         multi-megabyte data URL on every clock tick and drag frame. -->
    <Transition name="fade">
      <div v-if="config.wallpaper" class="desktop-wallpaper" aria-hidden="true">
        <img :src="config.wallpaper" alt="" decoding="async" />
      </div>
    </Transition>
    <aside id="app-sidebar" class="desktop-sidebar" aria-label="侧栏">
      <div class="sidebar-top-space" data-tauri-drag-region aria-hidden="true"></div>
      <SidebarPet
        :state="petState"
        :message="petMessage"
        :variant="petVariant"
        @check="checkNetwork"
      />
      <nav ref="navigation" class="desktop-navigation" aria-label="主导航">
        <span
          class="nav-indicator"
          :class="{ animated: navIndicator.animated.value }"
          :style="navIndicator.style.value"
          aria-hidden="true"
        ></span>
        <TransitionGroup
          tag="div"
          name="nav-swap"
          class="nav-section"
          :class="{ 'nav-dragging': !!draggedEntry }"
        >
          <template v-for="item in sidebarItems" :key="item.id">
            <button
              v-if="item.kind === 'entry'"
              class="desktop-nav-item"
              :class="{
                selected: route === item.id,
                dragging: draggedEntry === item.id,
              }"
              :data-indicator="item.id"
              :data-nav-id="item.id"
              :title="item.entry.title"
              :aria-label="item.entry.title"
              :aria-current="route === item.id ? 'page' : undefined"
              @pointerdown="startSidebarDrag($event, item.entry)"
              @click="activateNav(item.id)"
            >
              <EntryIcon :name="item.entry.iconName" :icon="item.entry.icon" :size="20" /><span>{{
                item.entry.title
              }}</span>
            </button>
            <div v-else class="nav-divider" role="separator" aria-hidden="true"></div>
          </template>
        </TransitionGroup>
        <div class="nav-spacer"></div>
        <div class="nav-section">
          <button
            v-for="item in footerItems"
            :key="item.id"
            class="desktop-nav-item"
            :class="{ selected: route === item.id }"
            :data-indicator="item.id"
            :title="item.title"
            :aria-label="item.title"
            :aria-current="route === item.id ? 'page' : undefined"
            @click="activateNav(item.id)"
          >
            <EntryIcon :name="item.iconName" :icon="item.icon" :size="20" /><span>{{
              item.title
            }}</span>
            <i v-if="item.id === 'update'" class="update-dot" aria-hidden="true"></i>
          </button>
        </div>
      </nav>
      <div class="sidebar-footer">
        <button
          class="desktop-identity"
          :title="identity"
          :aria-label="identity + '，个人页面'"
          :aria-current="route === 'personal' || route === 'accounts' ? 'page' : undefined"
          :class="{ selected: route === 'personal' || route === 'accounts' }"
          @click="route = 'personal'"
        >
          <UiAvatar :src="displayAvatar" :size="32" />
          <span class="desktop-identity-text">
            <strong>{{ displayName }}</strong>
            <small>{{
              account.connected ? account.studentId || '校园账号已连接' : '个人资料'
            }}</small>
          </span>
        </button>
      </div>
    </aside>
    <div
      class="sidebar-resizer"
      role="separator"
      aria-label="调整侧栏宽度"
      aria-orientation="vertical"
      aria-controls="app-sidebar"
      :aria-valuemin="collapsedWidth"
      :aria-valuemax="320"
      :aria-valuenow="sidebarWidth"
      :aria-valuetext="sidebarCollapsed ? '已收起，向右拖动展开' : sidebarWidth + ' 像素'"
      :aria-disabled="!configLoaded"
      :tabindex="configLoaded ? 0 : -1"
      title="拖动调整宽度，拖到底自动收起，双击恢复默认"
      @pointerdown="resize.start"
      @pointermove="resize.move"
      @pointerup="resize.finish"
      @pointercancel="resize.cancel"
      @lostpointercapture="resize.cancel"
      @dblclick="resize.reset"
      @keydown="resize.key"
    ></div>
    <button
      class="sidebar-toggle"
      :aria-label="config.sidebarCollapsed ? '展开侧栏' : '收起侧栏'"
      :title="
        (config.sidebarCollapsed ? '展开侧栏' : '收起侧栏') +
        (macNative ? '（⌘\\）' : '（Ctrl+\\）')
      "
      :aria-expanded="!config.sidebarCollapsed"
      aria-controls="app-sidebar"
      :disabled="!configLoaded"
      @click="toggleSidebar"
    >
      <ChevronLeft :size="14" class="sidebar-toggle-icon" />
    </button>
    <main class="desktop-main">
      <Transition name="pet-toast">
        <div
          v-if="toast"
          :key="toast.id"
          class="pet-toast"
          :class="`is-${toast.tone}`"
          role="status"
        >
          <Wifi
            v-if="toast.tone === 'campus' || toast.tone === 'vpn'"
            :size="14"
            aria-hidden="true"
          />
          <WifiOff v-else :size="14" aria-hidden="true" />
          <span>{{ toast.message }}</span>
        </div>
      </Transition>
      <!-- The macOS title bar is embedded, so this strip keeps the window draggable. -->
      <div v-if="macNative" class="titlebar-drag" data-tauri-drag-region aria-hidden="true"></div>
      <!-- Cycles the wallpaper presets. Icon only: the label lives in the tooltip
           and in 设置 → 外观与交互, so this never competes with page actions. -->
      <button
        v-if="showWallpaper"
        type="button"
        class="wallpaper-switch"
        :title="`显示样式：${wallpaperPreset.label} · 点击切换`"
        :aria-label="`显示样式：${wallpaperPreset.label}，点击切换`"
        @click="cycleWallpaper"
      >
        <Contrast :size="15" aria-hidden="true" />
      </button>
      <div ref="content" class="desktop-content">
        <div
          class="desktop-content-inner"
          :class="{
            'is-home': route === 'home',
            'is-wide':
              route === 'workbench' ||
              activePlugin?.page.type === 'campus' ||
              activePlugin?.page.type === 'pptCapture',
          }"
        >
          <Transition name="drop">
            <div v-if="error" class="notice desktop-error" role="alert">
              <AlertCircle :size="15" /><span>{{ error }}</span
              ><UiIconButton
                variant="inherit"
                size="xs"
                label="关闭提示"
                class="notice-close"
                @click="error = ''"
              >
                <X :size="14" />
              </UiIconButton>
            </div>
          </Transition>
          <Transition name="page" mode="out-in" @after-leave="resetScroll"
            ><div :key="route" class="route-content">
              <EmptyState v-if="!ready" :icon="LoaderCircle" busy title="正在载入…" />
              <HomeView
                v-else-if="route === 'home'"
                :config="config"
                :plugins="sortedPlugins"
                :connected="account.connected"
                :display-name="account.connected ? displayName : ''"
                :now="now"
                :has-timetable="!!timetable"
                :timetable-enabled="timetableEnabled"
                :courses="todayCourses"
                :notices="notices"
                :notices-loading="noticesLoading"
                :notices-error="noticesError"
                :loading="scheduleLoading"
                :error="scheduleError"
                :saving="saving || !configLoaded"
                @link="openLink"
                @refresh="refreshTimetable"
                @refresh-notices="refreshNotices"
                @toggle="togglePlugin"
                @account="route = 'accounts'"
                @open="route = $event"
              />
              <PersonalView
                v-else-if="route === 'personal'"
                :config="config"
                :account="account"
                :saving="saving"
                @update="persist"
                @accounts="route = 'accounts'"
              />
              <SettingsView
                v-else-if="route === 'settings'"
                :initial-section="settingsSection"
                :config="config"
                :saving="saving"
                @update="persist"
              />
              <CommunityView v-else-if="route === 'community'" />
              <WorkbenchView v-else-if="route === 'workbench'" :key="campusScope" />
              <AgentView
                v-else-if="route === 'agent'"
                @settings="route = 'cn.xjtu.agent-settings'"
              />
              <AgentSettingsView
                v-else-if="activePlugin?.page.type === 'agentSettings'"
                :key="activePlugin.id"
                :config="config"
                :saving="saving || !configLoaded"
                @agent="route = 'agent'"
                @update="persist"
              />
              <HomeThemeView
                v-else-if="activePlugin?.page.type === 'homeTheme'"
                :key="activePlugin.id"
                :config="config"
                :saving="saving || !configLoaded"
                @update="persist"
              />
              <div v-else-if="route === 'accounts'" class="subpage">
                <UiButton variant="ghost" size="sm" class="back-button" @click="route = 'personal'">
                  <ArrowLeft :size="14" /><span>个人</span>
                </UiButton>
                <AccountsView @changed="reloadAccount" />
              </div>
              <PluginMarketView
                v-else-if="route === 'market'"
                :plugins="sortedPlugins"
                :config="config"
                :saving="saving || !configLoaded"
                @toggle="togglePlugin"
              />
              <TimetableView
                v-else-if="activePlugin?.page.type === 'timetable'"
                :timetable="timetable"
                :connected="account.connected"
                :loading="scheduleLoading"
                :error="scheduleError"
                @refresh="refreshTimetable"
                @account="route = 'accounts'"
                @updated="timetable = $event"
              />
              <CampusView
                v-else-if="activePlugin?.page.type === 'campus'"
                :key="campusScope"
                :name="activePlugin.name"
                :avatar="displayAvatar"
                :preset="activePlugin.page.preset ?? 'all'"
                :access-mode="account.accessMode ?? null"
                :direct-session="account.directSession ?? false"
                @direct-session="refreshAccountStatus"
                @capture="openCapture"
              />
              <DesktopPetView v-else-if="activePlugin?.page.type === 'desktopPet'" />
              <FontBeautifyView
                v-else-if="activePlugin?.page.type === 'fontBeautify'"
                :settings="fontSettings"
                @update="fontSettings = $event"
              />
              <PptCaptureView
                v-else-if="activePlugin?.page.type === 'pptCapture'"
                :key="activePlugin.id"
                :target="captureTarget"
                :scope="account.studentId ?? account.name ?? ''"
              />
              <LinksView
                v-else-if="activePlugin?.page.type === 'links'"
                :key="activePlugin.id"
                :name="activePlugin.name"
                :links="activePlugin.page.links"
                @open="openLink"
              />
            </div>
          </Transition>
        </div>
      </div>
    </main>
  </div>
  <UpdateDialog :open="updateDialogOpen" @close="updateDialogOpen = false" />
</template>
