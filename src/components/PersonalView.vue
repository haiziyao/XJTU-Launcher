<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import EditableText from './EditableText.vue';
import { Camera, Copy, Globe, GraduationCap, ImagePlus, Plug, RefreshCw } from 'lucide-vue-next';
import type {
  AccountStatus,
  AppConfig,
  ExposeStatus,
  ExposeTool,
  ImageCrop,
  PersonalImageKind,
} from '../types';
import { bridge } from '../bridge';
import ImageCropDialog from './ImageCropDialog.vue';
import PageHeader from './ui/PageHeader.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import UiAvatar from './ui/UiAvatar.vue';
import IconTile from './ui/IconTile.vue';
import UiButton from './ui/UiButton.vue';
import UiSwitch from './ui/UiSwitch.vue';
const props = defineProps<{ config: AppConfig; account: AccountStatus; saving: boolean }>();
const emit = defineEmits<{ update: [config: AppConfig]; accounts: [] }>();
const error = ref('');
const importing = ref(false);
const cropping = ref<{ kind: PersonalImageKind; path: string; preview: string } | null>(null);

// Local MCP endpoint. The status comes from the Rust side so the page never
// claims the listener is up when the port could not be opened.
const status = ref<ExposeStatus>({
  running: false,
  enabled: false,
  address: 'http://127.0.0.1:39217/mcp',
  error: null,
});
const tools = ref<ExposeTool[]>([]);
const showTools = ref(false);
const busy = ref(false);
const probing = ref(false);
const probeState = ref<{ tone: 'ok' | 'error'; text: string } | null>(null);
const serverError = ref('');
const copied = ref('');
const portDraft = ref(String(props.config.mcpServerPort));
let copyTimer: number | undefined;
const command = computed(() => `claude mcp add --transport http xjtu ${status.value.address}`);
const openCount = computed(() => tools.value.filter((tool) => isToolOpen(tool.name)).length);
const portChanged = computed(() => {
  const port = Number(portDraft.value);
  if (!Number.isInteger(port) || port < 1024 || port > 65535) return false;
  return port !== props.config.mcpServerPort || !status.value.running;
});
/** Categories keep the 14 rows scannable without adding another control. */
const toolGroups = computed(() => {
  const order: string[] = [];
  const groups = new Map<string, ExposeTool[]>();
  for (const tool of tools.value) {
    if (!groups.has(tool.category)) {
      groups.set(tool.category, []);
      order.push(tool.category);
    }
    groups.get(tool.category)?.push(tool);
  }
  return order.map((title) => ({ title, tools: groups.get(title) ?? [] }));
});

async function loadServer() {
  try {
    status.value = await bridge.exposeStatus();
  } catch (e) {
    serverError.value = String(e);
  }
}
async function loadTools() {
  try {
    tools.value = await bridge.exposeTools();
  } catch (e) {
    serverError.value = String(e);
  }
}
async function toggleServer() {
  if (busy.value) return;
  busy.value = true;
  serverError.value = '';
  const enabled = !props.config.mcpServerEnabled;
  try {
    status.value = await bridge.setExposeEnabled(enabled);
    emit('update', { ...props.config, mcpServerEnabled: enabled });
    if (!enabled) {
      showTools.value = false;
      probeState.value = null;
    }
  } catch (e) {
    // The config keeps the previous value, so the switch falls back with it.
    serverError.value = String(e);
    await loadServer();
  } finally {
    busy.value = false;
  }
}
/** Startup restore can fail before the page exists; let the user try again. */
async function retryStart() {
  if (busy.value) return;
  busy.value = true;
  serverError.value = '';
  try {
    status.value = await bridge.setExposeEnabled(true);
  } catch (e) {
    serverError.value = String(e);
    await loadServer();
  } finally {
    busy.value = false;
  }
}
/** Restart on the typed port; the backend keeps the old listener if bind fails. */
async function applyPort() {
  if (busy.value || !portChanged.value) return;
  const port = Number(portDraft.value);
  busy.value = true;
  serverError.value = '';
  try {
    status.value = await bridge.setExposePort(port);
    emit('update', { ...props.config, mcpServerPort: port });
  } catch (e) {
    serverError.value = String(e);
    portDraft.value = String(props.config.mcpServerPort);
    await loadServer();
  } finally {
    busy.value = false;
  }
}
function isToolOpen(name: string) {
  return !props.config.mcpServerDisabledTools.includes(name);
}
function onToolToggle(name: string, event: Event) {
  const open = (event.target as HTMLInputElement).checked;
  const next = new Set(props.config.mcpServerDisabledTools);
  if (open) next.delete(name);
  else next.add(name);
  emit('update', { ...props.config, mcpServerDisabledTools: [...next] });
}
/** Cheap, honest per-tool state: the listener, the selection, then the session. */
function toolState(name: string): { tone: 'ok' | 'warn' | 'muted'; text: string } {
  if (!props.config.mcpServerEnabled || !status.value.running)
    return { tone: 'muted', text: '服务未启动' };
  if (!isToolOpen(name)) return { tone: 'muted', text: '未开放' };
  // The calendar is public; every other tool needs the current campus session.
  if (name !== 'campus_calendar' && !props.account.connected)
    return { tone: 'warn', text: '需登录校园账号' };
  return { tone: 'ok', text: '可达' };
}
async function probe() {
  probing.value = true;
  probeState.value = null;
  try {
    await bridge.probeExpose();
    probeState.value = { tone: 'ok', text: '校园会话可用' };
  } catch (e) {
    probeState.value = { tone: 'error', text: `校园会话不可用：${String(e)}` };
  } finally {
    probing.value = false;
  }
}
async function copy(text: string, key: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    // WKWebView may refuse the async clipboard without a permission grant.
    const area = document.createElement('textarea');
    area.value = text;
    area.setAttribute('readonly', '');
    area.style.position = 'fixed';
    area.style.opacity = '0';
    document.body.append(area);
    area.select();
    const copiedNow = document.execCommand('copy');
    area.remove();
    if (!copiedNow) {
      serverError.value = '无法复制到剪贴板';
      return;
    }
  }
  copied.value = key;
  window.clearTimeout(copyTimer);
  copyTimer = window.setTimeout(() => (copied.value = ''), 1600);
}
onMounted(() => {
  void loadServer();
  void loadTools();
});

async function choose(kind: PersonalImageKind) {
  importing.value = true;
  error.value = '';
  try {
    const picked = await bridge.pickPersonalImage(kind);
    if (picked) cropping.value = { kind, ...picked };
  } catch (e) {
    error.value = String(e);
  } finally {
    importing.value = false;
  }
}
function closeCrop() {
  if (cropping.value) URL.revokeObjectURL(cropping.value.preview);
  cropping.value = null;
}
async function applyCrop(crop: ImageCrop) {
  const target = cropping.value;
  if (!target) return;
  importing.value = true;
  try {
    const image = await bridge.cropPersonalImage(target.path, target.kind, crop);
    emit('update', { ...props.config, [target.kind]: image });
  } catch (e) {
    error.value = String(e);
  } finally {
    closeCrop();
    importing.value = false;
  }
}
onBeforeUnmount(() => {
  closeCrop();
  window.clearTimeout(copyTimer);
});
</script>
<template>
  <section class="desktop-page narrow-page is-centered">
    <PageHeader title="个人" description="本机个人资料与主页外观" />
    <Transition name="drop">
      <p v-if="error" class="notice" role="alert">{{ error }}</p>
    </Transition>

    <div class="profile-card">
      <button
        class="profile-avatar"
        title="更换个人头像"
        aria-label="更换个人头像"
        :disabled="saving || importing"
        @click="choose('avatar')"
      >
        <UiAvatar :src="config.avatar || account.avatar" :size="64" alt="个人头像" />
        <span class="profile-avatar-overlay" aria-hidden="true"><Camera :size="18" /></span>
      </button>
      <div class="profile-copy">
        <strong>{{
          config.nickname || account.name || (account.connected ? '校园账号' : '未登录')
        }}</strong>
        <small>{{ account.studentId || '本机个人资料' }}</small>
      </div>
      <div class="row-actions">
        <UiButton
          variant="ghost"
          size="sm"
          v-if="config.avatar"
          :disabled="saving || importing"
          @click="emit('update', { ...config, avatar: null })"
        >
          恢复默认
        </UiButton>
        <UiButton size="sm" :disabled="saving || importing" @click="choose('avatar')">
          更换头像
        </UiButton>
      </div>
    </div>

    <SettingGroup title="资料">
      <SettingRow title="显示名称" description="显示在侧栏与主页问候中">
        <EditableText
          :value="config.nickname"
          :placeholder="account.name || '使用校园账号姓名'"
          label="显示名称"
          :disabled="saving"
          :maxlength="40"
          @save="emit('update', { ...config, nickname: $event })"
        />
      </SettingRow>
    </SettingGroup>

    <SettingGroup title="主页壁纸">
      <div class="wallpaper-row">
        <div class="wallpaper-preview" :class="{ empty: !config.wallpaper }">
          <Transition name="fade" mode="out-in">
            <img
              v-if="config.wallpaper"
              :key="config.wallpaper"
              :src="config.wallpaper"
              alt="当前主页壁纸"
            />
            <span v-else>默认背景</span>
          </Transition>
        </div>
        <div class="wallpaper-copy">
          <strong>{{ config.wallpaper ? '自定义壁纸' : '默认背景' }}</strong>
          <small>壁纸铺满整个窗口，侧栏与卡片会自动变为半透明。</small>
          <div class="row-actions">
            <UiButton size="sm" :disabled="saving || importing" @click="choose('wallpaper')">
              <ImagePlus :size="14" />选择图片
            </UiButton>
            <UiButton
              variant="ghost"
              size="sm"
              v-if="config.wallpaper"
              :disabled="saving || importing"
              @click="emit('update', { ...config, wallpaper: null })"
            >
              移除
            </UiButton>
          </div>
        </div>
      </div>
    </SettingGroup>

    <SettingGroup title="校园身份">
      <SettingRow
        clickable
        title="校园账号"
        :description="account.connected ? '管理已保存的账号、头像与当前身份' : '尚未登录'"
        @select="emit('accounts')"
      >
        <template #leading>
          <IconTile tone="blue" :size="30"><GraduationCap :size="16" /></IconTile>
        </template>
        <span v-if="account.connected" class="badge success">已连接</span>
      </SettingRow>
    </SettingGroup>

    <SettingGroup
      title="开放接口"
      footnote="仅在本机监听，不经过启动器账号；本机任何程序都能调用已开放的工具。"
    >
      <SettingRow
        title="MCP 模式"
        description="让 Claude Code 等编程 Agent 连接后直接查询课表、成绩、考试与校园卡。"
      >
        <template #leading>
          <IconTile tone="blue" :size="30"><Plug :size="16" /></IconTile>
        </template>
        <UiSwitch
          :checked="config.mcpServerEnabled"
          label="MCP 模式"
          :disabled="saving || busy"
          @toggle="toggleServer"
        />
      </SettingRow>
      <p v-if="serverError && !config.mcpServerEnabled" class="connect-error">
        {{ serverError }}
      </p>
      <div v-if="config.mcpServerEnabled" class="connect-detail">
        <div class="connect-line">
          <span class="connect-label">端口</span>
          <input
            v-model="portDraft"
            class="text-field connect-port"
            inputmode="numeric"
            maxlength="5"
            aria-label="MCP 端口"
            :disabled="busy"
          />
          <UiButton size="sm" :disabled="busy || !portChanged" @click="applyPort">应用</UiButton>
        </div>
        <div v-if="!status.running" class="connect-error">
          <span>{{ status.error || serverError || 'MCP 服务未启动' }}</span>
          <UiButton variant="ghost" size="sm" :disabled="busy" @click="retryStart">
            <RefreshCw :size="13" />重试
          </UiButton>
        </div>
        <template v-else>
          <div class="connect-line">
            <span class="connect-label">地址</span>
            <code>{{ status.address }}</code>
            <UiButton variant="ghost" size="sm" @click="copy(status.address, 'address')">
              <Copy :size="13" />{{ copied === 'address' ? '已复制' : '复制' }}
            </UiButton>
          </div>
          <div class="connect-line">
            <span class="connect-label">命令</span>
            <code>{{ command }}</code>
            <UiButton variant="ghost" size="sm" @click="copy(command, 'command')">
              <Copy :size="13" />{{ copied === 'command' ? '已复制' : '复制' }}
            </UiButton>
          </div>
          <div class="connect-tools">
            <span>已开放 {{ openCount }} 个工具 · 仅本机访问</span>
            <UiButton variant="ghost" size="sm" @click="showTools = !showTools">
              {{ showTools ? '收起工具' : '查看工具' }}
            </UiButton>
          </div>
          <div v-if="showTools" class="tool-panel">
            <div class="tool-head">
              <span>未勾选的工具不会出现在 tools/list 中</span>
              <UiButton variant="ghost" size="sm" :disabled="probing" @click="probe">
                {{ probing ? '检查中…' : '检查可达性' }}
              </UiButton>
            </div>
            <p v-if="probeState" class="tool-probe" :class="probeState.tone">
              {{ probeState.text }}
            </p>
            <div v-for="group in toolGroups" :key="group.title" class="tool-group">
              <p class="tool-group-title">{{ group.title }}</p>
              <label v-for="tool in group.tools" :key="tool.name" class="tool-row">
                <input
                  type="checkbox"
                  :checked="isToolOpen(tool.name)"
                  @change="onToolToggle(tool.name, $event)"
                />
                <span class="tool-copy">
                  <code>{{ tool.name }}</code>
                  <small>{{ tool.description }}</small>
                </span>
                <span class="tool-state" :class="toolState(tool.name).tone">
                  {{ toolState(tool.name).text }}
                </span>
              </label>
            </div>
          </div>
        </template>
      </div>
      <SettingRow title="Web 模式" description="在浏览器中查看校园数据。">
        <template #leading>
          <IconTile tone="blue" :size="30"><Globe :size="16" /></IconTile>
        </template>
        <span class="badge">尚未实现</span>
      </SettingRow>
    </SettingGroup>
    <ImageCropDialog
      v-if="cropping"
      :src="cropping.preview"
      :kind="cropping.kind"
      :busy="importing"
      @confirm="applyCrop"
      @cancel="closeCrop"
    />
  </section>
</template>
