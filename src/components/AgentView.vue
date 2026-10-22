<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { ask } from '@tauri-apps/plugin-dialog';
import {
  ArrowUp,
  AtSign,
  Check,
  ChevronDown,
  ChevronRight,
  Folder,
  LoaderCircle,
  PanelLeft,
  Plus,
  Settings2,
  Square,
  Wrench,
} from 'lucide-vue-next';
import { bridge } from '../bridge';
import type {
  AgentEvent,
  AgentMessage,
  AgentPresets,
  AgentSkill,
  AgentSummary,
  AgentWorkspace,
  AgentTask,
  AgentToolCall,
  AgentConversation,
  ModelProviders,
  ProviderBalance,
} from '../types';
import MenuSelect from './ui/MenuSelect.vue';
import AgentHistory from './AgentHistory.vue';

const emit = defineEmits<{ settings: [] }>();

interface LiveStep {
  callId: string;
  name: string;
  arguments: string;
  content: string;
  ok: boolean | null;
}

const TOOL_LABELS: Record<string, string> = {
  campus_timetable: '查询课程表',
  campus_terms: '查询学期',
  campus_grades: '查询成绩',
  campus_exams: '查询考试安排',
  campus_calendar: '查询校历',
  campus_attendance_streams: '查询考勤流水',
  campus_attendance_records: '查询考勤',
  campus_card: '查询校园卡',
  campus_card_transactions: '查询校园卡流水',
  campus_fitness: '查询体测',
  campus_fitness_score: '查询体测成绩',
  campus_room_options: '查询教室选项',
  campus_rooms: '查询空闲教室',
  campus_courses: '查询全校课程',
};

const providers = ref<ModelProviders>({
  defaultId: null,
  providers: [],
  legacyKeyAvailable: false,
});
const providerId = ref('');
const modelId = ref('');
const usage = ref<{ input: number; output: number } | null>(null);
const balance = ref<ProviderBalance | null>(null);
const showPlus = ref(false);
const showAt = ref(false);
const presets = ref<AgentPresets>({ version: 1, roles: [], skills: [], tasks: [] });
const roleId = ref('');
const selectedSkills = ref<AgentSkill[]>([]);
const conversation = ref<AgentConversation | null>(null);
const summaries = ref<AgentSummary[]>([]);
const workspaces = ref<AgentWorkspace[]>([]);
/** Workspace for the next new conversation, chosen from the drawer. */
const workspaceId = ref('');
const messages = ref<AgentMessage[]>([]);
const showHistory = ref(false);
const showWorkspaces = ref(false);
const renaming = ref(false);
const renameTitle = ref('');
const titleInput = ref<HTMLInputElement | null>(null);
const draft = ref('');
const running = ref(false);
const liveText = ref('');
const liveSteps = ref<LiveStep[]>([]);
const expanded = ref(new Set<string>());
const error = ref('');
const scroller = ref<HTMLElement | null>(null);
let requestId = '';
let modelSave: Promise<void> = Promise.resolve();
let unlisten: (() => void) | undefined;

/** One flat list: every cached model of every provider. */
const modelOptions = computed(() =>
  providers.value.providers.flatMap((provider) => {
    const list = provider.models.length ? provider.models : provider.model ? [provider.model] : [];
    return list.map((model) => ({
      value: `${provider.id}|${model}`,
      label: model,
    }));
  }),
);
const modelChoice = computed(() =>
  providerId.value && modelId.value ? `${providerId.value}|${modelId.value}` : '',
);
const activeProvider = computed(
  () => providers.value.providers.find((provider) => provider.id === providerId.value) ?? null,
);
const usedTokens = computed(() => (usage.value ? usage.value.input + usage.value.output : 0));
const contextWindow = computed(() => contextWindowFor(modelId.value));
const ringRatio = computed(() => Math.min(1, usedTokens.value / contextWindow.value));
const ringOffset = computed(() => 2 * Math.PI * 8 * (1 - ringRatio.value));
const balanceText = computed(() => {
  if (!balance.value) return '';
  const symbol =
    balance.value.currency === 'CNY' ? '¥' : balance.value.currency === 'USD' ? '$' : '';
  const amount = Number(balance.value.amount);
  return symbol + (Number.isFinite(amount) ? amount.toFixed(2) : balance.value.amount);
});

/** Approximate context windows; vendors do not expose them through /models. */
function contextWindowFor(model: string): number {
  const name = model.toLowerCase();
  if (name.includes('gemini')) return 1_000_000;
  if (name.includes('claude')) return 200_000;
  if (name.includes('deepseek')) return 64_000;
  return 128_000;
}
function formatTokens(value: number): string {
  return value >= 1000 ? `${(value / 1000).toFixed(1)}k` : String(value);
}
const roleOptions = computed(() => [
  { value: '', label: '通用角色' },
  ...presets.value.roles.map((role) => ({ value: role.id, label: role.name })),
]);
const slashQuery = computed(() => {
  const text = draft.value;
  if (!text.startsWith('/') || text.includes('\n')) return null;
  return text.slice(1).trim();
});
const slashMatches = computed(() => {
  const query = slashQuery.value;
  if (query === null || !presets.value.skills.length) return [];
  const needle = query.toLowerCase();
  return presets.value.skills.filter(
    (skill) =>
      skill.name.toLowerCase().includes(needle) || skill.description.toLowerCase().includes(needle),
  );
});
const currentTitle = computed(() => conversation.value?.title || '新对话');
const activeWorkspaceName = computed(
  () => workspaces.value.find((workspace) => workspace.id === workspaceId.value)?.name || '默认',
);
const canSend = computed(
  () =>
    !!draft.value.trim() && !running.value && !!providerId.value && modelOptions.value.length > 0,
);

function errorText(value: unknown) {
  return value instanceof Error ? value.message : String(value);
}
async function scrollDown() {
  await nextTick();
  const node = scroller.value;
  if (node) node.scrollTop = node.scrollHeight;
}
async function loadProviders() {
  try {
    providers.value = await bridge.modelProviders();
    providerId.value = providers.value.defaultId || providers.value.providers[0]?.id || '';
    const provider = providers.value.providers.find((item) => item.id === providerId.value);
    if (provider) {
      const models = provider.models ?? [];
      modelId.value =
        models.includes(provider.model) || !models.length ? provider.model : models[0];
    }
    void loadBalance();
  } catch (e) {
    error.value = errorText(e);
  }
}
async function loadPresets() {
  try {
    presets.value = await bridge.agentPresets();
  } catch (e) {
    error.value = errorText(e);
  }
}
async function loadBalance() {
  balance.value = null;
  if (!providerId.value) return;
  try {
    balance.value = await bridge.providerBalance(providerId.value);
  } catch {
    balance.value = null;
  }
}
/** The chat page owns the real model choice; it is persisted before sending. */
async function changeModel(value: string) {
  const index = value.indexOf('|');
  if (index < 0) return;
  const provider = value.slice(0, index);
  const model = value.slice(index + 1);
  if (!provider || !model) return;
  providerId.value = provider;
  modelId.value = model;
  modelSave = bridge.setProviderModel(provider, model).catch(() => {
    // The selection still travels with the next request.
  });
  await modelSave;
  void loadBalance();
}
function pickRole(value: string) {
  showPlus.value = false;
  void changeRole(value);
}
function pickTask(task: AgentTask) {
  showPlus.value = false;
  applyTask(task);
}
function pickSkill(skill: AgentSkill) {
  showAt.value = false;
  attachSkill(skill);
}
function closeMenus() {
  showHistory.value = false;
  showPlus.value = false;
  showAt.value = false;
  showWorkspaces.value = false;
}
async function changeRole(value: string) {
  roleId.value = value;
  if (conversation.value) {
    try {
      await bridge.agentSetRole(conversation.value.id, value || null);
      conversation.value.roleId = value || null;
      await loadSummaries();
    } catch (e) {
      error.value = errorText(e);
    }
  }
}
function attachSkill(skill: AgentSkill) {
  if (!selectedSkills.value.some((item) => item.id === skill.id)) {
    selectedSkills.value = [...selectedSkills.value, skill];
  }
  draft.value = '';
}
function detachSkill(id: string) {
  selectedSkills.value = selectedSkills.value.filter((skill) => skill.id !== id);
}
function applyTask(task: AgentTask) {
  draft.value = task.prompt;
  roleId.value = task.roleId || '';
  selectedSkills.value = presets.value.skills.filter((skill) => task.skillIds.includes(skill.id));
}
async function renameConversation(id: string, title: string) {
  try {
    await bridge.agentRename(id, title);
    if (conversation.value?.id === id) conversation.value.title = title;
    await loadSummaries();
  } catch (e) {
    error.value = errorText(e);
  }
}
async function pinConversation(id: string, pinned: boolean) {
  try {
    await bridge.agentPin(id, pinned);
    if (conversation.value?.id === id) conversation.value.pinned = pinned;
    await loadSummaries();
  } catch (e) {
    error.value = errorText(e);
  }
}
async function loadWorkspaces() {
  try {
    workspaces.value = await bridge.agentWorkspaces();
  } catch (e) {
    error.value = errorText(e);
  }
}
async function loadSummaries() {
  try {
    summaries.value = await bridge.agentList();
  } catch (e) {
    error.value = errorText(e);
  }
}
async function openConversation(id: string) {
  if (running.value) return;
  try {
    const next = await bridge.agentGet(id);
    conversation.value = next;
    messages.value = next.messages.slice();
    roleId.value = next.roleId || '';
    workspaceId.value = next.workspaceId || '';
    selectedSkills.value = [];
    liveText.value = '';
    liveSteps.value = [];
    usage.value = null;
    expanded.value = new Set();
    error.value = '';
    showHistory.value = false;
    await scrollDown();
  } catch (e) {
    error.value = errorText(e);
  }
}
function newChat() {
  if (running.value) void stop();
  workspaceId.value = '';
  conversation.value = null;
  messages.value = [];
  roleId.value = '';
  selectedSkills.value = [];
  liveText.value = '';
  liveSteps.value = [];
  usage.value = null;
  expanded.value = new Set();
  draft.value = '';
  error.value = '';
  showHistory.value = false;
}
/** Moves the open conversation, or just picks the target for the next one. */
async function chooseWorkspace(id: string) {
  showWorkspaces.value = false;
  workspaceId.value = id;
  if (!conversation.value) return;
  try {
    await bridge.agentSetWorkspace(conversation.value.id, id || null);
    conversation.value.workspaceId = id || null;
    await loadSummaries();
  } catch (e) {
    error.value = errorText(e);
  }
}
/** The header title renames the open conversation in place. */
function startRenameTitle() {
  if (!conversation.value || running.value) return;
  renameTitle.value = conversation.value.title;
  renaming.value = true;
  void nextTick(() => {
    const input = titleInput.value;
    if (!input) return;
    input.focus();
    input.select();
  });
}
async function commitTitle() {
  if (!renaming.value) return;
  renaming.value = false;
  const title = renameTitle.value.trim();
  const target = conversation.value;
  if (!target || !title || title === target.title) return;
  try {
    await bridge.agentRename(target.id, title);
    target.title = title;
    await loadSummaries();
  } catch (e) {
    error.value = errorText(e);
  }
}

function startInWorkspace(id: string | null) {
  newChat();
  workspaceId.value = id || '';
  showHistory.value = false;
}
async function createWorkspace(name: string) {
  try {
    await bridge.agentCreateWorkspace(name);
    await loadWorkspaces();
  } catch (e) {
    error.value = errorText(e);
  }
}
async function removeConversation(summary: AgentSummary) {
  if (
    !(await ask(`删除对话“${summary.title}”？`, {
      title: '删除对话',
      kind: 'warning',
      okLabel: '删除',
      cancelLabel: '取消',
    }))
  )
    return;
  try {
    await bridge.agentDelete(summary.id);
    if (conversation.value?.id === summary.id) newChat();
    await loadSummaries();
  } catch (e) {
    error.value = errorText(e);
  }
}
function onEvent(event: AgentEvent) {
  if (event.requestId !== requestId) return;
  switch (event.type) {
    case 'delta':
      liveText.value += event.text;
      void scrollDown();
      break;
    case 'toolStart':
      liveSteps.value.push({
        callId: event.callId,
        name: event.name,
        arguments: event.arguments,
        content: '',
        ok: null,
      });
      void scrollDown();
      break;
    case 'usage':
      usage.value = { input: event.inputTokens, output: event.outputTokens };
      break;
    case 'toolEnd': {
      const step = liveSteps.value.find((item) => item.callId === event.callId);
      if (step) {
        step.ok = event.ok;
        step.content = event.content;
      }
      break;
    }
    case 'done':
    case 'cancelled':
      void settle();
      break;
    case 'error':
      error.value = event.message;
      void settle();
      break;
  }
}
async function settle() {
  running.value = false;
  liveText.value = '';
  liveSteps.value = [];
  void loadBalance();
  if (conversation.value) {
    try {
      const fresh = await bridge.agentGet(conversation.value.id);
      conversation.value = fresh;
      messages.value = fresh.messages.slice();
      await loadSummaries();
    } catch (e) {
      error.value = errorText(e);
    }
  }
  await scrollDown();
}
async function send() {
  const text = draft.value.trim();
  if (!text || running.value) return;
  if (!providerId.value) {
    error.value = '请先在 Agent 设置中配置模型服务';
    return;
  }
  error.value = '';
  if (!conversation.value) {
    try {
      conversation.value = await bridge.agentNew(workspaceId.value || null);
    } catch (e) {
      error.value = errorText(e);
      return;
    }
  }
  draft.value = '';
  messages.value.push({ role: 'user', content: text });
  liveText.value = '';
  liveSteps.value = [];
  running.value = true;
  requestId =
    typeof crypto.randomUUID === 'function'
      ? crypto.randomUUID()
      : `req-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  const skillIds = selectedSkills.value.map((skill) => skill.id);
  selectedSkills.value = [];
  await scrollDown();
  await modelSave;
  try {
    if (roleId.value !== (conversation.value.roleId || '')) {
      await bridge.agentSetRole(conversation.value.id, roleId.value || null);
      conversation.value.roleId = roleId.value || null;
    }
    await bridge.agentSend({
      conversationId: conversation.value.id,
      requestId,
      content: text,
      providerId: providerId.value || null,
      model: modelId.value || null,
      skillIds,
    });
  } catch (e) {
    error.value = errorText(e);
    running.value = false;
  }
}
async function stop() {
  if (!running.value) return;
  try {
    await bridge.agentCancel(requestId);
  } catch (e) {
    error.value = errorText(e);
  }
}
function resultFor(message: AgentMessage, call: AgentToolCall): AgentMessage | undefined {
  return messages.value.find(
    (candidate) => candidate.role === 'tool' && candidate.toolCallId === call.id,
  );
}
function callLabel(name: string) {
  if (TOOL_LABELS[name]) return TOOL_LABELS[name];
  if (name.startsWith('mcp__')) {
    const parts = name.split('__');
    return `${parts[1] || 'MCP'} · ${parts.slice(2).join('__')}`;
  }
  return name;
}
function toggleTool(callId: string) {
  const next = new Set(expanded.value);
  if (next.has(callId)) next.delete(callId);
  else next.add(callId);
  expanded.value = next;
}
function prettyArguments(raw: string) {
  try {
    const value = JSON.parse(raw);
    if (!value || (typeof value === 'object' && !Object.keys(value).length)) return '';
    return JSON.stringify(value, null, 2);
  } catch {
    return raw;
  }
}
function shortContent(text: string, limit = 6000) {
  return text.length > limit ? `${text.slice(0, limit)}\n…` : text;
}
watch(liveText, () => void scrollDown());
onMounted(async () => {
  await Promise.all([loadProviders(), loadSummaries(), loadPresets(), loadWorkspaces()]);
  try {
    unlisten = await bridge.onAgent(onEvent);
  } catch (e) {
    error.value = errorText(e);
  }
});
onBeforeUnmount(() => {
  unlisten?.();
});
</script>
<template>
  <section class="desktop-page agent-page">
    <header class="agent-bar">
      <div class="agent-bar-title">
        <button
          class="agent-icon-button"
          :class="{ active: showHistory }"
          title="对话管理"
          aria-label="对话管理"
          :aria-expanded="showHistory"
          @click="showHistory = !showHistory"
        >
          <PanelLeft :size="16" />
        </button>
        <div class="agent-workspace-picker">
          <button
            class="agent-workspace"
            :class="{ active: showWorkspaces }"
            :aria-expanded="showWorkspaces"
            title="当前工作区"
            aria-label="选择工作区"
            @click="showWorkspaces = !showWorkspaces"
          >
            <Folder :size="13" />
            <span>{{ activeWorkspaceName }}</span>
            <ChevronDown :size="12" />
          </button>
          <div v-if="showWorkspaces" class="agent-workspace-menu" role="menu">
            <button
              type="button"
              role="menuitem"
              :class="{ current: !workspaceId }"
              @click="chooseWorkspace('')"
            >
              默认
            </button>
            <button
              v-for="workspace in workspaces"
              :key="workspace.id"
              type="button"
              role="menuitem"
              :class="{ current: workspaceId === workspace.id }"
              @click="chooseWorkspace(workspace.id)"
            >
              {{ workspace.name }}
            </button>
          </div>
        </div>
        <input
          v-if="renaming"
          ref="titleInput"
          v-model="renameTitle"
          class="agent-title-input"
          :style="{ width: `${Math.min(16, Math.max(4, renameTitle.length + 1))}em` }"
          maxlength="60"
          autofocus
          aria-label="对话名称"
          @keydown.enter.prevent="commitTitle"
          @keydown.esc="renaming = false"
          @blur="commitTitle"
        />
        <button
          v-else-if="conversation"
          class="agent-name clickable"
          title="点击重命名对话"
          @click="startRenameTitle"
        >
          {{ currentTitle }}
        </button>
        <span v-else class="agent-name">{{ currentTitle }}</span>
      </div>
      <div class="agent-bar-actions">
        <button class="agent-icon-button" title="新对话" aria-label="新对话" @click="newChat">
          <Plus :size="15" />
        </button>
        <button
          class="agent-icon-button"
          title="Agent 设置"
          aria-label="Agent 设置"
          @click="emit('settings')"
        >
          <Settings2 :size="15" />
        </button>
      </div>
    </header>

    <Transition name="drop">
      <p v-if="error" class="notice agent-notice" role="alert">{{ error }}</p>
    </Transition>

    <div
      v-if="showHistory || showPlus || showAt || showWorkspaces"
      class="agent-backdrop"
      @click="closeMenus"
    ></div>
    <Transition name="drop">
      <AgentHistory
        v-if="showHistory"
        :summaries="summaries"
        :workspaces="workspaces"
        :current-id="conversation?.id"
        :disabled="running"
        @open="openConversation"
        @remove="removeConversation"
        @rename="renameConversation"
        @pin="pinConversation"
        @close="showHistory = false"
        @create="startInWorkspace"
        @create-workspace="createWorkspace"
      />
    </Transition>

    <div ref="scroller" class="agent-thread">
      <div class="agent-column">
        <div v-if="!messages.length && !running" class="agent-empty">
          <h2>有什么可以帮你？</h2>
          <p v-if="modelOptions.length">
            可以查询课表、成绩、考试、考勤、校园卡，也会调用已启用的 MCP 工具。
          </p>
          <p v-else>还没有可用的模型服务，先到 Agent 设置里添加。</p>
          <button v-if="!modelOptions.length" class="btn" @click="emit('settings')">
            打开 Agent 设置
          </button>
          <div v-if="presets.tasks.length" class="agent-tasks">
            <p class="agent-tasks-title">预设任务</p>
            <button
              v-for="task in presets.tasks"
              :key="task.id"
              class="agent-task"
              @click="applyTask(task)"
            >
              <strong>{{ task.name }}</strong>
              <span>{{ task.description || task.prompt }}</span>
            </button>
          </div>
        </div>

        <template v-for="(message, index) in messages" :key="index">
          <div v-if="message.role === 'user'" class="agent-user">{{ message.content }}</div>
          <div v-else-if="message.role === 'assistant'" class="agent-assistant">
            <div v-if="message.content" class="agent-text">{{ message.content }}</div>
            <div v-for="call in message.toolCalls" :key="call.id" class="agent-tool">
              <button
                class="agent-tool-row"
                :aria-expanded="expanded.has(call.id)"
                @click="toggleTool(call.id)"
              >
                <Wrench :size="13" class="agent-tool-icon" />
                <span class="agent-tool-name">{{ callLabel(call.name) }}</span>
                <span
                  class="agent-tool-status"
                  :class="{ failed: resultFor(message, call)?.error }"
                >
                  <template v-if="resultFor(message, call)?.error">失败</template>
                  <template v-else-if="resultFor(message, call)">完成</template>
                  <template v-else>执行中</template>
                </span>
                <ChevronRight
                  :size="13"
                  class="agent-tool-chevron"
                  :class="{ open: expanded.has(call.id) }"
                />
              </button>
              <div v-if="expanded.has(call.id)" class="agent-tool-detail">
                <pre v-if="prettyArguments(call.arguments)">{{
                  prettyArguments(call.arguments)
                }}</pre>
                <pre
                  v-if="resultFor(message, call)"
                  :class="{ failed: resultFor(message, call)?.error }"
                  >{{ shortContent(resultFor(message, call)!.content) }}</pre>
              </div>
            </div>
          </div>
        </template>

        <div v-if="running || liveText || liveSteps.length" class="agent-assistant">
          <div v-if="liveText" class="agent-text">{{ liveText }}</div>
          <div v-for="step in liveSteps" :key="step.callId" class="agent-tool">
            <button
              class="agent-tool-row"
              :aria-expanded="expanded.has(step.callId)"
              @click="toggleTool(step.callId)"
            >
              <Wrench :size="13" class="agent-tool-icon" />
              <span class="agent-tool-name">{{ callLabel(step.name) }}</span>
              <span class="agent-tool-status" :class="{ failed: step.ok === false }">
                <template v-if="step.ok === null">执行中</template>
                <template v-else-if="step.ok">完成</template>
                <template v-else>失败</template>
              </span>
              <ChevronRight
                :size="13"
                class="agent-tool-chevron"
                :class="{ open: expanded.has(step.callId) }"
              />
            </button>
            <div v-if="expanded.has(step.callId)" class="agent-tool-detail">
              <pre v-if="prettyArguments(step.arguments)">{{
                prettyArguments(step.arguments)
              }}</pre>
              <pre v-if="step.content" :class="{ failed: step.ok === false }">{{
                shortContent(step.content)
              }}</pre>
            </div>
          </div>
          <div v-if="running && !liveText && !liveSteps.length" class="agent-thinking">
            <LoaderCircle :size="13" class="spin" />正在思考
          </div>
        </div>
      </div>
    </div>

    <div class="agent-composer-wrap">
      <div class="agent-column">
        <form class="agent-composer" @submit.prevent="send">
          <div v-if="slashMatches.length" class="agent-slash">
            <button
              v-for="skill in slashMatches"
              :key="skill.id"
              type="button"
              @click="attachSkill(skill)"
            >
              <strong>{{ skill.name }}</strong>
              <span>{{ skill.description }}</span>
            </button>
          </div>
          <div v-if="selectedSkills.length" class="agent-chips">
            <span v-for="skill in selectedSkills" :key="skill.id" class="agent-chip">
              {{ skill.name }}
              <button type="button" aria-label="移除 Skill" @click="detachSkill(skill.id)">
                ×
              </button>
            </span>
          </div>
          <textarea
            v-model="draft"
            rows="1"
            :placeholder="modelOptions.length ? '问点什么…' : '请先配置模型服务'"
            :disabled="!modelOptions.length"
            aria-label="消息"
            @keydown.enter.exact.prevent="send"
          ></textarea>
          <div class="agent-composer-bar">
            <MenuSelect
              v-if="modelOptions.length"
              :model-value="modelChoice"
              :options="modelOptions"
              label="选择模型"
              class="agent-model-select"
              @update:model-value="changeModel"
            />
            <span
              v-if="modelChoice"
              class="agent-usage"
              :title="`上下文 ${formatTokens(usedTokens)} / ${formatTokens(contextWindow)}`"
            >
              <svg viewBox="0 0 20 20" aria-hidden="true">
                <circle class="usage-track" cx="10" cy="10" r="8" />
                <circle
                  class="usage-value"
                  :class="{ full: ringRatio > 0.9 }"
                  cx="10"
                  cy="10"
                  r="8"
                  :style="{
                    strokeDasharray: `${2 * Math.PI * 8}`,
                    strokeDashoffset: `${ringOffset}`,
                  }"
                />
              </svg>
            </span>
            <span
              v-if="balanceText"
              class="agent-balance"
              :title="`${activeProvider?.name ?? ''} 余额`"
              >{{ balanceText }}</span
            >
            <span v-if="running" class="agent-running"><Check :size="12" />生成中</span>
            <span v-else></span>
            <button
              type="button"
              class="agent-icon-button"
              :class="{ active: showPlus }"
              title="预设与角色"
              aria-label="预设与角色"
              :aria-expanded="showPlus"
              @click="
                showPlus = !showPlus;
                showAt = false;
              "
            >
              <Plus :size="15" />
            </button>
            <button
              type="button"
              class="agent-icon-button"
              :class="{ active: showAt }"
              title="使用 Skill"
              aria-label="使用 Skill"
              :aria-expanded="showAt"
              @click="
                showAt = !showAt;
                showPlus = false;
              "
            >
              <AtSign :size="15" />
            </button>
            <button
              v-if="running"
              type="button"
              class="agent-send-circle stop"
              aria-label="停止生成"
              @click="stop"
            >
              <Square :size="12" />
            </button>
            <button
              v-else
              type="submit"
              class="agent-send-circle"
              aria-label="发送"
              :disabled="!canSend"
            >
              <ArrowUp :size="15" />
            </button>
          </div>
          <div v-if="showPlus" class="agent-menu agent-plus-menu" role="menu">
            <p class="agent-menu-label">角色</p>
            <button
              v-for="option in roleOptions"
              :key="option.value"
              type="button"
              role="menuitem"
              @click="pickRole(option.value)"
            >
              <strong>{{ option.label }}</strong>
              <span v-if="option.value === roleId">当前</span>
            </button>
            <template v-if="presets.tasks.length">
              <p class="agent-menu-label">预设任务</p>
              <button
                v-for="task in presets.tasks"
                :key="task.id"
                type="button"
                role="menuitem"
                @click="pickTask(task)"
              >
                <strong>{{ task.name }}</strong>
                <span>{{ task.description || task.prompt }}</span>
              </button>
            </template>
          </div>
          <div v-if="showAt" class="agent-menu agent-at-menu" role="menu">
            <p class="agent-menu-label">Skill</p>
            <p v-if="!presets.skills.length" class="agent-menu-empty">还没有创建 Skill</p>
            <button
              v-for="skill in presets.skills"
              :key="skill.id"
              type="button"
              role="menuitem"
              @click="pickSkill(skill)"
            >
              <strong>{{ skill.name }}</strong>
              <span>{{ skill.description }}</span>
            </button>
          </div>
        </form>
        <p class="agent-disclaimer">Agent 可能出错，校园数据请以学校系统为准。</p>
      </div>
    </div>
  </section>
</template>
<style scoped>
.agent-page {
  position: relative;
  display: flex;
  flex-direction: column;
  height: calc(100vh - 70px);
  min-height: 420px;
}
.mac-native .agent-page {
  height: calc(100vh - 100px);
}
.agent-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--border);
}
.agent-bar-title {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}
.agent-workspace-picker {
  position: relative;
  flex-shrink: 0;
}
.agent-workspace {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 24px;
  padding: 0 6px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--text-tertiary);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
.agent-workspace span {
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.agent-workspace:hover,
.agent-workspace.active {
  background: var(--accent-wash);
  color: var(--accent-text);
}
.agent-workspace-menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  z-index: 22;
  min-width: 170px;
  max-height: 260px;
  overflow-y: auto;
  display: grid;
  gap: 2px;
  padding: 4px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface);
  box-shadow: 0 12px 30px rgba(15, 23, 42, 0.12);
}
.agent-workspace-menu button {
  border: 0;
  border-radius: 7px;
  background: transparent;
  padding: 6px 8px;
  text-align: left;
  font: inherit;
  font-size: 12.5px;
  color: var(--text);
  cursor: pointer;
}
.agent-workspace-menu button:hover {
  background: var(--accent-wash);
}
.agent-workspace-menu button.current {
  background: var(--accent-soft);
  color: var(--accent-text);
}
.agent-name {
  font-size: 17px;
  font-weight: 600;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.agent-name.clickable {
  border: 0;
  border-radius: 6px;
  background: transparent;
  padding: 2px 5px;
  font: inherit;
  font-size: 17px;
  font-weight: 600;
  color: var(--text);
  cursor: text;
}
.agent-name.clickable:hover {
  background: var(--accent-wash);
}
.agent-title-input {
  max-width: 280px;
  padding: 2px 5px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--text);
  caret-color: var(--accent);
  cursor: text;
  font: inherit;
  font-size: 17px;
  font-weight: 600;
  outline: none;
}
.agent-bar-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}
.agent-icon-button {
  width: 28px;
  height: 28px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--text-tertiary);
  display: grid;
  place-items: center;
  cursor: pointer;
}
.agent-icon-button:hover:not(:disabled) {
  background: var(--surface-hover);
  color: var(--text);
}
.agent-icon-button.active {
  background: var(--surface-pressed);
  color: var(--text);
}
.agent-notice {
  margin: 10px 0 0;
}
.agent-backdrop {
  position: fixed;
  inset: 0;
  z-index: 20;
}
.agent-thread {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 18px 0 8px;
}
.agent-column {
  width: 100%;
  max-width: 720px;
  margin: 0 auto;
  padding: 0 4px;
}
.agent-empty {
  padding: 18vh 0 0;
  text-align: center;
  color: var(--text-secondary);
}
.agent-empty h2 {
  margin: 0 0 8px;
  font-size: 18px;
  font-weight: 600;
  color: var(--text);
}
.agent-empty p {
  margin: 0 0 14px;
  font-size: 13px;
  line-height: 1.6;
}
.agent-user {
  margin: 18px 0 0 auto;
  max-width: 82%;
  width: fit-content;
  padding: 8px 13px;
  border-radius: 14px;
  background: var(--accent-soft);
  color: var(--text);
  font-size: 14px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
}
.agent-assistant {
  margin: 16px 0 0;
  font-size: 14px;
  line-height: 1.7;
  color: var(--text);
}
.agent-text {
  white-space: pre-wrap;
  word-break: break-word;
}
.agent-tool {
  margin-top: 6px;
}
.agent-tool-row {
  display: flex;
  align-items: center;
  gap: 7px;
  width: 100%;
  border: 0;
  background: transparent;
  padding: 5px 6px;
  border-radius: 7px;
  color: var(--text-secondary);
  font: inherit;
  font-size: 12.5px;
  text-align: left;
  cursor: pointer;
}
.agent-tool-row:hover {
  background: var(--accent-wash);
}
.agent-tool-icon {
  color: var(--text-tertiary);
  flex-shrink: 0;
}
.agent-tool-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.agent-tool-status {
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.agent-tool-status.failed {
  color: var(--danger);
}
.agent-tool-chevron {
  color: var(--text-tertiary);
  transition: transform 0.15s ease;
}
.agent-tool-chevron.open {
  transform: rotate(90deg);
}
.agent-tool-detail {
  margin: 2px 0 8px 26px;
  border-left: 2px solid var(--border);
  padding-left: 10px;
}
.agent-tool-detail pre {
  margin: 4px 0;
  font-family: var(--font-mono);
  font-size: 11.5px;
  line-height: 1.55;
  color: var(--text-secondary);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 240px;
  overflow-y: auto;
}
.agent-tool-detail pre.failed {
  color: var(--danger);
}
.agent-thinking {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-top: 8px;
  color: var(--text-tertiary);
  font-size: 12.5px;
}
.agent-composer-wrap {
  position: relative;
  z-index: 22;
  padding: 8px 0 0;
}
.agent-composer {
  position: relative;
  border-radius: 14px;
  background: var(--surface);
  box-shadow: 0 6px 20px rgba(15, 23, 42, 0.06);
  padding: 10px 10px 8px 14px;
}
.agent-composer:focus-within {
  box-shadow: 0 0 0 3px var(--accent-tint);
}
.agent-composer textarea {
  width: 100%;
  border: 0;
  resize: none;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: 14px;
  line-height: 1.55;
  min-height: 24px;
  max-height: 160px;
  padding: 2px 0;
}
.agent-composer textarea:focus {
  outline: none;
}
.agent-composer-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 4px;
}
.agent-model-select {
  max-width: 260px;
}
.agent-model-select :deep(.field-shell) {
  border: 0;
  background: transparent;
  box-shadow: none;
  padding: 0;
}
.agent-model-select :deep(.menu-field-value) {
  font-size: 12px;
  color: var(--text-secondary);
  padding: 2px 4px;
  border-radius: 6px;
}
.agent-model-select :deep(.menu-field-value:hover) {
  background: var(--surface-hover);
}
.agent-running {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.agent-composer-bar > span:empty {
  margin-left: auto;
}
.agent-send-circle {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: 0;
  background: var(--accent);
  color: var(--accent-contrast);
  display: grid;
  place-items: center;
  cursor: pointer;
}
.agent-send-circle:disabled {
  background: var(--surface-sunken);
  color: var(--text-tertiary);
  cursor: default;
}
.agent-send-circle.stop {
  background: var(--surface-sunken);
  color: var(--text);
}
.agent-tasks {
  margin: 26px auto 0;
  max-width: 520px;
  display: grid;
  gap: 8px;
  text-align: left;
}
.agent-tasks-title {
  margin: 0 0 2px;
  font-size: 12px;
  color: var(--text-tertiary);
}
.agent-task {
  display: grid;
  gap: 2px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface);
  text-align: left;
  font: inherit;
  cursor: pointer;
}
.agent-task:hover {
  border-color: var(--border-strong);
  background: var(--surface-subtle);
}
.agent-task strong {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}
.agent-task span {
  font-size: 12px;
  color: var(--text-tertiary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.agent-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding-bottom: 8px;
}
.agent-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 4px 2px 8px;
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent-text);
  font-size: 12px;
}
.agent-chip button {
  border: 0;
  background: transparent;
  color: inherit;
  cursor: pointer;
  font-size: 13px;
  line-height: 1;
  padding: 2px;
  border-radius: 50%;
}
.agent-chip button:hover {
  background: var(--accent-tint);
}
.agent-slash {
  position: absolute;
  left: 8px;
  right: 8px;
  bottom: calc(100% + 6px);
  max-height: 220px;
  overflow-y: auto;
  display: grid;
  gap: 2px;
  padding: 4px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface);
  box-shadow: 0 12px 30px rgba(15, 23, 42, 0.12);
  z-index: 5;
}
.agent-slash button {
  display: grid;
  gap: 2px;
  border: 0;
  background: transparent;
  padding: 7px 9px;
  border-radius: 7px;
  text-align: left;
  font: inherit;
  cursor: pointer;
}
.agent-slash button:hover {
  background: var(--surface-hover);
}
.agent-slash strong {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}
.agent-slash span {
  font-size: 11.5px;
  color: var(--text-tertiary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.agent-usage {
  display: inline-flex;
  align-items: center;
  margin-left: 2px;
}
.agent-usage svg {
  width: 16px;
  height: 16px;
  transform: rotate(-90deg);
}
.agent-usage circle {
  fill: none;
  stroke-width: 2.4;
}
.usage-track {
  stroke: var(--border);
}
.usage-value {
  stroke: var(--accent);
  stroke-linecap: round;
  transition: stroke-dashoffset var(--duration) var(--ease-out);
}
.usage-value.full {
  stroke: var(--danger);
}
.agent-balance {
  margin-left: 4px;
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
.agent-menu {
  position: absolute;
  right: 8px;
  bottom: calc(100% + 6px);
  z-index: 6;
  min-width: 220px;
  max-width: 320px;
  max-height: 260px;
  overflow-y: auto;
  display: grid;
  gap: 2px;
  padding: 6px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface);
  box-shadow: 0 12px 30px rgba(15, 23, 42, 0.12);
}
.agent-menu-label {
  margin: 4px 6px 2px;
  color: var(--text-tertiary);
  font-size: 11px;
}
.agent-menu-empty {
  margin: 2px 6px 4px;
  color: var(--text-tertiary);
  font-size: 12px;
}
.agent-menu button {
  display: grid;
  gap: 2px;
  border: 0;
  background: transparent;
  padding: 6px 8px;
  border-radius: 7px;
  text-align: left;
  font: inherit;
  cursor: pointer;
}
.agent-menu button:hover {
  background: var(--surface-hover);
}
.agent-menu strong {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text);
}
.agent-menu span {
  font-size: 11.5px;
  color: var(--text-tertiary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.agent-disclaimer {
  margin: 6px 0 0;
  text-align: center;
  color: var(--text-tertiary);
  font-size: 11px;
}

/* With a wallpaper the page stays translucent so the photo is not blocked. */
.desktop-shell.has-wallpaper .agent-composer {
  background: var(--panel);
}
/* Popovers keep the wallpaper visible instead of covering it with white. */
.desktop-shell.has-wallpaper .agent-menu,
.desktop-shell.has-wallpaper .agent-slash,
.desktop-shell.has-wallpaper .agent-workspace-menu {
  border-color: var(--panel-border);
  background: var(--panel);
}
.desktop-shell.has-wallpaper .agent-user {
  background-color: rgba(255, 255, 255, 0.9);
  background-image: linear-gradient(var(--accent-tint), var(--accent-tint));
}
.desktop-shell.has-wallpaper .agent-assistant {
  padding: 8px 12px;
  border-radius: 14px;
  background-color: rgba(255, 255, 255, 0.86);
  background-image: linear-gradient(var(--accent-wash), var(--accent-wash));
}
.desktop-shell.has-wallpaper .agent-task {
  border-color: var(--panel-border);
  background: var(--panel);
}
.desktop-shell.has-wallpaper .agent-task:hover {
  background: var(--surface-hover);
}
</style>
