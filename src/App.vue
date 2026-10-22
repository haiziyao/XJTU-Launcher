<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import {
  AlertCircle,
  ArrowLeft,
  Bot,
  CalendarDays,
  ChevronLeft,
  Compass,
  Home,
  LayoutGrid,
  LoaderCircle,
  MessagesSquare,
  Settings,
  Wifi,
  WifiOff,
  X,
} from 'lucide-vue-next';
import { initializeMarket } from './market';
import { initializeUpdates } from './updates';
import { bridge, native } from './bridge';
import { coursesForDay, defaultConfig, moveItem, orderedPlugins, visiblePlugins } from './domain';
import type {
  AccountStatus,
  AppConfig,
  NetworkCheck,
  PetState,
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
import AgentView from './components/AgentView.vue';
import { useSidebarResize } from './useSidebarResize';
import PluginMarketView from './components/PluginMarketView.vue';
import LinksView from './components/LinksView.vue';
import TimetableView from './components/TimetableView.vue';
import EmptyState from './components/ui/EmptyState.vue';
import UiAvatar from './components/ui/UiAvatar.vue';
import { useSlidingIndicator } from './useSlidingIndicator';
import { applyAccent, normalizeAccent } from './theme';
import './ui.css';
import './pages.css';
import './desktop-shell.css';
import './pet-toast.css';
import './pet-art.css';

const route = ref('home');
const navigation = ref<HTMLElement | null>(null);
const content = ref<HTMLElement | null>(null);
const navIndicator = useSlidingIndicator(navigation, () => route.value);
const primaryNav = [
  { id: 'home', title: '主页', icon: Home },
  { id: 'agent', title: 'Agent', icon: Bot },
  { id: 'community', title: '社区', icon: MessagesSquare },
];
// Start each page at the top once the previous one has faded out.
function resetScroll() {
  content.value?.scrollTo({ top: 0 });
}
const settingsSection = ref<'appearance' | 'home' | 'models' | 'plugins' | 'updates'>('appearance');
function openSettings(
  section: 'appearance' | 'home' | 'models' | 'plugins' | 'updates' = 'appearance',
) {
  settingsSection.value = section;
  route.value = 'settings';
}
const config = ref(defaultConfig());
const configLoaded = ref(false);
const petVariant = computed(() => normalizePet(config.value.pet));
// Accent tokens live on the document root so teleported dialogs inherit them too.
watch(() => config.value.accent, applyAccent, { immediate: true });
const pluginList = ref<PluginList>({ plugins: [], warnings: [], directory: '', sources: {} });
const account = ref<AccountStatus>({ connected: false, degree: null, apiKeySet: false });
const timetable = ref<Timetable | null>(null);
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
// The embedded macOS title bar paints the window buttons over the rail's top-left.
const sidebarCollapsedWidth = macNative ? 76 : 56;
const sortedPlugins = computed(() => orderedPlugins(pluginList.value.plugins, config.value));
const enabledPlugins = computed(() =>
  sortedPlugins.value.filter((p) => !config.value.disabledPlugins.includes(p.id)),
);
const sidebarPlugins = computed(() => visiblePlugins(sortedPlugins.value, config.value));
const activePlugin = computed(() => enabledPlugins.value.find((p) => p.id === route.value));
watch(enabledPlugins, () => {
  if (
    !['home', 'settings', 'market', 'personal', 'accounts', 'community', 'agent'].includes(
      route.value,
    ) &&
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
  try {
    const next = await bridge.account();
    if (!disposed && generation === accountGeneration) account.value = next;
  } catch (e) {
    error.value = errorText(e);
  }
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
  try {
    const result = await bridge.checkNetwork();
    message = result.message;
    tone = result.tone;
    next = result.tone === 'campus' || result.tone === 'vpn' ? 'ok' : 'warn';
  } catch (e) {
    message = errorText(e);
  }
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
async function openLink(url: string) {
  try {
    await bridge.openUrl(url);
  } catch (e) {
    error.value = errorText(e);
  }
}
function toggleVisibility(id: string) {
  void persist({
    ...config.value,
    hiddenPlugins: config.value.hiddenPlugins.includes(id)
      ? config.value.hiddenPlugins.filter((p) => p !== id)
      : [...config.value.hiddenPlugins, id],
  });
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
    if (!disposed && generation === accountGeneration && timetableEnabled.value)
      timetable.value = result;
  } catch (e) {
    if (!disposed && generation === accountGeneration) scheduleError.value = errorText(e);
  } finally {
    scheduleLoading.value = false;
  }
}
onMounted(async () => {
  if (!native) return;
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
  if (results[2].status === 'fulfilled') account.value = results[2].value;
  else error.value = errorText(results[2].reason);
  try {
    const stop = await bridge.onAuth(async (event) => {
      if (disposed) return;
      if (event.state === 'success') {
        const generation = ++accountGeneration;
        timetable.value = null;
        scheduleError.value = '';
        try {
          const next = await bridge.account();
          if (!disposed && generation === accountGeneration) account.value = next;
        } catch (e) {
          error.value = errorText(e);
        }
      } else if (event.state === 'error') error.value = event.message;
    });
    if (disposed) stop();
    else unlisten = stop;
  } catch (e) {
    error.value = errorText(e);
  }
  ready.value = true;
  void initializeUpdates(config.value.checkAppUpdates).catch((e) => (error.value = errorText(e)));
  void initializeMarket(reloadPlugins).catch((e) => (error.value = errorText(e)));
});
onBeforeUnmount(() => {
  disposed = true;
  unlisten?.();
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
      'motion-ready': ready,
      'reduce-motion': config.reduceMotion,
    }"
    :style="{
      '--sidebar-width': sidebarWidth + 'px',
      '--sidebar-collapsed-width': collapsedWidth + 'px',
    }"
  >
    <aside id="app-sidebar" class="desktop-sidebar" aria-label="侧栏">
      <div class="sidebar-top-space" aria-hidden="true"></div>
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
            v-for="item in primaryNav"
            :key="item.id"
            class="desktop-nav-item"
            :class="{ selected: route === item.id }"
            :data-indicator="item.id"
            :title="item.title"
            :aria-label="item.title"
            :aria-current="route === item.id ? 'page' : undefined"
            @click="route = item.id"
          >
            <component :is="item.icon" :size="20" /><span>{{ item.title }}</span>
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
            <CalendarDays v-if="plugin.page.type === 'timetable'" :size="20" /><Compass
              v-else
              :size="20"
            /><span>{{ plugin.name }}</span>
          </button>
        </div>
        <div class="nav-spacer"></div>
        <div class="nav-section">
          <button
            class="desktop-nav-item"
            :class="{ selected: route === 'market' }"
            data-indicator="market"
            title="插件市场"
            aria-label="插件市场"
            :aria-current="route === 'market' ? 'page' : undefined"
            @click="route = 'market'"
          >
            <LayoutGrid :size="20" /><span>插件市场</span>
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
            <Settings :size="20" /><span>应用设置</span>
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
    <main class="desktop-main" :class="{ 'has-wallpaper': route === 'home' && config.wallpaper }">
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
      <Transition name="fade">
        <div
          v-if="route === 'home' && config.wallpaper"
          class="desktop-wallpaper"
          :style="{ backgroundImage: `url(${config.wallpaper})` }"
          aria-hidden="true"
        ></div>
      </Transition>
      <div ref="content" class="desktop-content">
        <div class="desktop-content-inner" :class="{ 'is-home': route === 'home' }">
          <Transition name="drop">
            <div v-if="error" class="notice desktop-error" role="alert">
              <AlertCircle :size="15" /><span>{{ error }}</span
              ><button class="notice-close" aria-label="关闭提示" @click="error = ''">
                <X :size="14" />
              </button>
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
                :loading="scheduleLoading"
                :error="scheduleError"
                :saving="saving || !configLoaded"
                @link="openLink"
                @refresh="refreshTimetable"
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
              <AgentView v-else-if="route === 'agent'" @models="openSettings('models')" />
              <div v-else-if="route === 'accounts'" class="subpage">
                <button class="back-button" @click="route = 'personal'">
                  <ArrowLeft :size="14" /><span>个人</span>
                </button>
                <AccountsView @changed="reloadAccount" />
              </div>
              <PluginMarketView
                v-else-if="route === 'market'"
                :plugins="sortedPlugins"
                :config="config"
                :saving="saving || !configLoaded"
                :warnings="pluginList.warnings"
                :sources="pluginList.sources"
                @visibility="toggleVisibility"
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
              />
              <LinksView
                v-else-if="activePlugin?.page.type === 'links'"
                :key="activePlugin.id"
                :name="activePlugin.name"
                :links="activePlugin.page.links"
                @open="openLink"
              /></div
          ></Transition>
        </div>
      </div>
    </main>
  </div>
</template>
