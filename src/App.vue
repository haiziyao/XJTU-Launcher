<script setup lang="ts">
import { computed, defineAsyncComponent, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import {
  AlertCircle,
  ArrowLeft,
  Bot,
  CalendarDays,
  ChevronLeft,
  Compass,
  Contrast,
  Download,
  Home,
  LayoutGrid,
  LoaderCircle,
  MapPinned,
  MessagesSquare,
  Presentation,
  Route,
  Briefcase,
  Settings,
  Palette,
  Settings2,
  Wifi,
  WifiOff,
  X,
} from 'lucide-vue-next';
import { initializeMarket } from './market';
import { initializeUpdates, updateStatus } from './updates';
import { bridge, native } from './bridge';
import { ensureDirectSession } from './campusSession';
import { prefetchCampus } from './campusPrefetch';
import { resetLmsOverview } from './campusOverview';
import {
  coursesForDay,
  defaultConfig,
  moveItem,
  orderedPlugins,
  sidebarPlugins as pluginsForSidebar,
} from './domain';
import type {
  AccountStatus,
  AppConfig,
  CampusNoticeBoard,
  NetworkCheck,
  PetState,
  Plugin,
  PluginList,
  Timetable,
} from './types';
import SidebarPet from './components/SidebarPet.vue';
import { normalizePet } from './pets';
import HomeView from './components/HomeView.vue';
import AccountsView from './components/AccountsView.vue';
import PersonalView from './components/PersonalView.vue';
import SettingsView from './components/SettingsView.vue';
import CommunityView from './components/CommunityView.vue';
import WorkbenchView from './components/WorkbenchView.vue';
import AgentView from './components/AgentView.vue';
import AgentSettingsView from './components/AgentSettingsView.vue';
import HomeThemeView from './components/HomeThemeView.vue';
import UpdateDialog from './components/UpdateDialog.vue';
import { useSidebarResize } from './useSidebarResize';
import PluginMarketView from './components/PluginMarketView.vue';
import LinksView from './components/LinksView.vue';
import TimetableView from './components/TimetableView.vue';
import { applyCached } from './cache';
import { loadEntryIcons } from './icons';
import CampusView from './components/CampusView.vue';
import PptCaptureView from './components/PptCaptureView.vue';
import type { CaptureTarget } from './capture/target';
import EmptyState from './components/ui/EmptyState.vue';
import EntryIcon from './components/ui/EntryIcon.vue';
import UiAvatar from './components/ui/UiAvatar.vue';
import UiButton from './components/ui/UiButton.vue';
import UiIconButton from './components/ui/UiIconButton.vue';
import { useSlidingIndicator } from './useSlidingIndicator';
import {
  WALLPAPER_MODES,
  applyAccent,
  normalizeAccent,
  wallpaperMode,
  wallpaperPanelOpacity,
  wallpaperTokens,
} from './theme';
import './ui.css';
import './pages.css';
import './campus.css';
import './desktop-shell.css';
import './pet-toast.css';
import './pet-art.css';

const route = ref('home');
/** Set when 思源学堂 hands a lesson to the capture page; that page reads it on mount. */
const captureTarget = ref<CaptureTarget | null>(null);
function openCapture(target: CaptureTarget) {
  const plugin = enabledPlugins.value.find((p) => p.page.type === 'pptCapture');
  if (!plugin) {
    showToast('回放截取已停用，先在「设置 → 插件」里启用', 'neutral');
    return;
  }
  captureTarget.value = target;
  route.value = plugin.id;
}
const navigation = ref<HTMLElement | null>(null);
const content = ref<HTMLElement | null>(null);
const navIndicator = useSlidingIndicator(navigation, () => route.value);
// 组件画廊只在开发构建里存在；生产包不会导入它，也不显示入口。
const KitGallery = import.meta.env.DEV
  ? defineAsyncComponent(() => import('./components/dev/KitGallery.vue'))
  : null;
// Start each page at the top once the previous one has faded out.
function resetScroll() {
  content.value?.scrollTo({ top: 0 });
}
const settingsSection = ref<'appearance' | 'home' | 'pet' | 'plugins' | 'updates'>('appearance');
function openSettings(
  section: 'appearance' | 'home' | 'pet' | 'plugins' | 'updates' = 'appearance',
) {
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
// Hiding the entry must not leave the app on a page with no way back.
watch(
  () => config.value.agentSidebarVisible,
  (visible) => {
    if (!visible && route.value === 'agent') route.value = 'home';
  },
  { immediate: true },
);
const pluginList = ref<PluginList>({ plugins: [], warnings: [], directory: '', sources: {} });
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
// 畅行校园固定在主导航里（社区下方）；其余插件仍归到「插件」分组。
const campusPlugin = computed(() =>
  enabledPlugins.value.find((plugin) => plugin.page.type === 'campus'),
);
const sidebarPlugins = computed(() =>
  pluginsForSidebar(sortedPlugins.value, config.value).filter(
    (plugin) => plugin.page.type !== 'campus',
  ),
);
// 主页 / Agent / 社区是固定入口；畅行校园跟随插件启用状态，工作台始终可用。
const primaryNav = computed(() => [
  { id: 'home', title: '主页', icon: Home },
  { id: 'agent', title: 'Agent', icon: Bot },
  { id: 'community', title: '社区', icon: MessagesSquare },
  ...(campusPlugin.value
    ? [{ id: campusPlugin.value.id, title: '畅行校园', icon: MapPinned }]
    : []),
  { id: 'workbench', title: '工作台', icon: Briefcase },
  ...(KitGallery ? [{ id: 'kit', title: '组件', icon: LayoutGrid }] : []),
]);
// Agent is hidden by default; the switch lives in Agent settings.
const visiblePrimaryNav = computed(() =>
  primaryNav.value.filter((item) => item.id !== 'agent' || config.value.agentSidebarVisible),
);
/** Built-in glyph per page type; a plugin cannot ship its own icon file. */
function pluginGlyph(plugin: Plugin) {
  switch (plugin.page.type) {
    case 'timetable':
      return CalendarDays;
    case 'campus':
      return Route;
    case 'homeTheme':
      return Palette;
    case 'pptCapture':
      return Presentation;
    case 'agentSettings':
      return Settings2;
    default:
      return Compass;
  }
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
      'kit',
    ].includes(route.value) &&
    !activePlugin.value
  )
    route.value = 'home';
});
watch(route, () => {
  if (activePlugin.value?.page.type === 'timetable' && !timetable.value) void refreshTimetable();
});
async function reloadPlugins() {
  try {
    pluginList.value = await bridge.plugins();
  } catch (e) {
    error.value = errorText(e);
  }
}
function movePlugin(index: number, delta: number) {
  void persist({
    ...config.value,
    pluginOrder: moveItem(
      sortedPlugins.value.map((p) => p.id),
      index,
      delta,
    ),
  });
}
async function reloadAccount() {
  const generation = ++accountGeneration;
  timetable.value = null;
  scheduleError.value = '';
  resetLmsOverview();
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
async function saveSource(source: string | null): Promise<boolean> {
  await persist({ ...config.value, pluginServer: source });
  return config.value.pluginServer === source;
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
        resetLmsOverview();
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
        if (!disposed) openSettings('pet');
      }),
      bridge.onPetConfigChanged((next) => {
        if (!disposed) config.value = next;
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
  void initializeMarket(reloadPlugins).catch((e) => (error.value = errorText(e)));
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
        <div class="nav-section">
          <button
            v-for="item in visiblePrimaryNav"
            :key="item.id"
            class="desktop-nav-item"
            :class="{ selected: route === item.id }"
            :data-indicator="item.id"
            :title="item.title"
            :aria-label="item.title"
            :aria-current="route === item.id ? 'page' : undefined"
            @click="route = item.id"
          >
            <EntryIcon :name="`nav.${item.id}`" :icon="item.icon" :size="20" /><span>{{
              item.title
            }}</span>
          </button>
        </div>
        <div v-if="sidebarPlugins.length" class="nav-section">
          <div class="sidebar-group-label"><span>插件</span></div>
          <button
            v-for="plugin in sidebarPlugins"
            :key="plugin.id"
            class="desktop-nav-item"
            :class="{ selected: route === plugin.id }"
            :data-indicator="plugin.id"
            :title="plugin.name"
            :aria-label="plugin.name"
            :aria-current="route === plugin.id ? 'page' : undefined"
            @click="route = plugin.id"
          >
            <EntryIcon
              :name="`plugin.${plugin.id}`"
              :icon="pluginGlyph(plugin)"
              :size="20"
            /><span>{{ plugin.name }}</span>
          </button>
        </div>
        <div class="nav-spacer"></div>
        <div class="nav-section">
          <button
            v-if="updateReady"
            class="desktop-nav-item update-entry"
            title="发现新版本"
            aria-label="发现新版本"
            @click="updateDialogOpen = true"
          >
            <Download :size="20" /><span>发现新版本</span
            ><i class="update-dot" aria-hidden="true"></i>
          </button>
          <button
            class="desktop-nav-item"
            :class="{ selected: route === 'market' }"
            data-indicator="market"
            title="插件市场"
            aria-label="插件市场"
            :aria-current="route === 'market' ? 'page' : undefined"
            @click="route = 'market'"
          >
            <EntryIcon name="nav.market" :icon="LayoutGrid" :size="20" /><span>插件市场</span>
          </button>
          <button
            class="desktop-nav-item"
            :class="{ selected: route === 'settings' }"
            data-indicator="settings"
            title="应用设置"
            aria-label="应用设置"
            :aria-current="route === 'settings' ? 'page' : undefined"
            @click="openSettings()"
          >
            <EntryIcon name="nav.settings" :icon="Settings" :size="20" /><span>应用设置</span>
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
                :save-source="saveSource"
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
                :warnings="pluginList.warnings"
                :sources="pluginList.sources"
                @settings="openSettings('plugins')"
                @toggle="togglePlugin"
                @move="movePlugin"
                @reload="reloadPlugins"
                @open="route = $event"
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
                :access-mode="account.accessMode ?? null"
                :direct-session="account.directSession ?? false"
                @direct-session="refreshAccountStatus"
                @capture="openCapture"
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
              <component v-else-if="KitGallery && route === 'kit'" :is="KitGallery" />
            </div>
          </Transition>
        </div>
      </div>
    </main>
  </div>
  <UpdateDialog :open="updateDialogOpen" @close="updateDialogOpen = false" />
</template>
