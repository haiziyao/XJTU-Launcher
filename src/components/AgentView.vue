<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { ask } from '@tauri-apps/plugin-dialog';
import {
  ArrowUp,
  Bot,
  Check,
  History,
  LoaderCircle,
  Plus,
  Settings2,
  Trash2,
  Wrench,
} from 'lucide-vue-next';
import { bridge } from '../bridge';
import type {
  AgentEvent,
  AgentMessage,
  AgentSummary,
  AgentToolCall,
  AgentConversation,
  ModelProviders,
} from '../types';
import PageHeader from './ui/PageHeader.vue';
import MenuSelect from './ui/MenuSelect.vue';
import EmptyState from './ui/EmptyState.vue';
import UiAvatar from './ui/UiAvatar.vue';

const emit = defineEmits<{ settings: [] }>();

interface LiveStep {
  callId: string;
  name: string;
  arguments: string;
  content: string;
  ok: boolean | null;
}

const providers = ref<ModelProviders>({
  defaultId: null,
  providers: [],
  legacyKeyAvailable: false,
});
const providerId = ref('');
const conversation = ref<AgentConversation | null>(null);
const summaries = ref<AgentSummary[]>([]);
const messages = ref<AgentMessage[]>([]);
const showHistory = ref(false);
const draft = ref('');
const running = ref(false);
const liveText = ref('');
const liveSteps = ref<LiveStep[]>([]);
const error = ref('');
const scroller = ref<HTMLElement | null>(null);
let requestId = '';
let unlisten: (() => void) | undefined;

const providerOptions = computed(() =>
  providers.value.providers.map((provider) => ({
    value: provider.id,
    label: `${provider.name} · ${provider.model}`,
  })),
);
const currentTitle = computed(() => conversation.value?.title || '新对话');
const canSend = computed(
  () =>
    !!draft.value.trim() &&
    !running.value &&
    !!providerId.value &&
    providerOptions.value.length > 0,
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
    liveText.value = '';
    liveSteps.value = [];
    error.value = '';
    showHistory.value = false;
    await scrollDown();
  } catch (e) {
    error.value = errorText(e);
  }
}
function newChat() {
  if (running.value) void stop();
  conversation.value = null;
  messages.value = [];
  liveText.value = '';
  liveSteps.value = [];
  draft.value = '';
  error.value = '';
  showHistory.value = false;
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
      conversation.value = await bridge.agentNew();
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
  requestId = crypto.randomUUID();
  await scrollDown();
  try {
    await bridge.agentSend({
      conversationId: conversation.value.id,
      requestId,
      content: text,
      providerId: providerId.value || null,
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
  if (name.startsWith('mcp__')) {
    const parts = name.split('__');
    return `${parts[1] || 'MCP'} · ${parts.slice(2).join('__')}`;
  }
  return name.replace(/^campus_/, '校园 · ');
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
function shortContent(text: string, limit = 4000) {
  return text.length > limit ? `${text.slice(0, limit)}\n…` : text;
}
watch(liveText, () => void scrollDown());
onMounted(async () => {
  await Promise.all([loadProviders(), loadSummaries()]);
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
    <PageHeader title="Agent">
      <template #actions>
        <MenuSelect
          v-if="providerOptions.length"
          v-model="providerId"
          :options="providerOptions"
          label="选择模型"
        />
        <button
          class="btn ghost icon"
          :aria-pressed="showHistory"
          title="历史记录"
          aria-label="历史记录"
          @click="showHistory = !showHistory"
        >
          <History :size="16" />
        </button>
        <button class="btn" @click="newChat"><Plus :size="14" />新对话</button>
        <button
          class="btn ghost icon"
          title="Agent 设置"
          aria-label="Agent 设置"
          @click="emit('settings')"
        >
          <Settings2 :size="16" />
        </button>
      </template>
    </PageHeader>

    <Transition name="drop">
      <p v-if="error" class="notice" role="alert">{{ error }}</p>
    </Transition>

    <div class="agent-layout">
      <aside v-if="showHistory" class="agent-history" aria-label="历史对话">
        <EmptyState
          v-if="!summaries.length"
          compact
          :icon="Bot"
          title="还没有对话"
          description="发送第一条消息后会出现在这里。"
        />
        <div v-for="summary in summaries" :key="summary.id" class="agent-history-row">
          <button
            class="agent-history-open"
            :class="{ current: conversation?.id === summary.id }"
            :disabled="running"
            @click="openConversation(summary.id)"
          >
            <strong>{{ summary.title }}</strong>
            <small>{{ summary.messages }} 条消息</small>
          </button>
          <button
            class="btn ghost icon small"
            :disabled="running"
            :aria-label="'删除 ' + summary.title"
            @click="removeConversation(summary)"
          >
            <Trash2 :size="14" />
          </button>
        </div>
      </aside>

      <div ref="scroller" class="agent-thread">
        <EmptyState
          v-if="!messages.length && !running"
          :icon="Bot"
          title="开始和 Agent 对话"
          :description="
            providerOptions.length
              ? '可以问课程表、成绩、考勤、校园卡，也会调用已启用的 MCP 工具。'
              : '还没有可用的模型服务，先到 Agent 设置里添加。'
          "
        >
          <button v-if="!providerOptions.length" class="btn" @click="emit('settings')">
            打开 Agent 设置
          </button>
        </EmptyState>
        <template v-for="(message, index) in messages" :key="index">
          <div v-if="message.role === 'user'" class="agent-row user">
            <div class="agent-bubble">{{ message.content }}</div>
            <UiAvatar :src="null" :size="28" />
          </div>
          <div v-else-if="message.role === 'assistant'" class="agent-row assistant">
            <div class="agent-avatar"><Bot :size="16" /></div>
            <div class="agent-assistant-body">
              <div v-if="message.content" class="agent-text">{{ message.content }}</div>
              <div v-for="call in message.toolCalls" :key="call.id" class="agent-tool">
                <header>
                  <Wrench :size="14" />
                  <strong>{{ callLabel(call.name) }}</strong>
                  <span class="agent-tool-state">
                    <Check
                      v-if="resultFor(message, call) && !resultFor(message, call)?.error"
                      :size="13"
                    />
                    <LoaderCircle v-else-if="!resultFor(message, call)" :size="13" class="spin" />
                    {{
                      resultFor(message, call)?.error
                        ? '失败'
                        : resultFor(message, call)
                          ? '完成'
                          : '执行中'
                    }}
                  </span>
                </header>
                <pre v-if="prettyArguments(call.arguments)" class="agent-tool-args">{{
                  prettyArguments(call.arguments)
                }}</pre>
                <pre
                  v-if="resultFor(message, call)"
                  class="agent-tool-result"
                  :class="{ failed: resultFor(message, call)?.error }"
                  >{{ shortContent(resultFor(message, call)!.content) }}</pre>
              </div>
            </div>
          </div>
        </template>
        <div v-if="running || liveText || liveSteps.length" class="agent-row assistant">
          <div class="agent-avatar"><Bot :size="16" /></div>
          <div class="agent-assistant-body">
            <div v-if="liveText" class="agent-text">{{ liveText }}</div>
            <div v-for="step in liveSteps" :key="step.callId" class="agent-tool">
              <header>
                <Wrench :size="14" />
                <strong>{{ callLabel(step.name) }}</strong>
                <span class="agent-tool-state">
                  <Check v-if="step.ok === true" :size="13" />
                  <LoaderCircle v-else-if="step.ok === null" :size="13" class="spin" />
                  {{ step.ok === null ? '执行中' : step.ok ? '完成' : '失败' }}
                </span>
              </header>
              <pre v-if="prettyArguments(step.arguments)" class="agent-tool-args">{{
                prettyArguments(step.arguments)
              }}</pre>
              <pre
                v-if="step.content"
                class="agent-tool-result"
                :class="{ failed: step.ok === false }"
                >{{ shortContent(step.content) }}</pre>
            </div>
            <div v-if="running && !liveText && !liveSteps.length" class="agent-pending">
              <LoaderCircle :size="14" class="spin" />正在思考…
            </div>
          </div>
        </div>
      </div>
    </div>

    <form class="agent-composer" @submit.prevent="send">
      <textarea
        v-model="draft"
        rows="1"
        :placeholder="
          providerOptions.length ? '发消息，Enter 发送，Shift+Enter 换行' : '请先配置模型服务'
        "
        :disabled="!providerOptions.length"
        aria-label="消息"
        @keydown.enter.exact.prevent="send"
      ></textarea>
      <button
        v-if="running"
        type="button"
        class="btn agent-send stop"
        aria-label="停止生成"
        @click="stop"
      >
        停止
      </button>
      <button v-else type="submit" class="btn primary agent-send" :disabled="!canSend">
        <ArrowUp :size="15" />发送
      </button>
    </form>
    <p class="agent-hint">{{ currentTitle }}</p>
  </section>
</template>
<style scoped>
.agent-page {
  display: flex;
  flex-direction: column;
  min-height: calc(100vh - 140px);
}
.agent-layout {
  display: flex;
  gap: 20px;
  flex: 1;
  min-height: 0;
}
.agent-history {
  width: 240px;
  flex-shrink: 0;
  border-right: 1px solid var(--border);
  padding-right: 12px;
  max-height: 60vh;
  overflow-y: auto;
}
.agent-history-row {
  display: flex;
  align-items: center;
  gap: 4px;
  border-radius: 10px;
}
.agent-history-row:hover {
  background: var(--surface-hover);
}
.agent-history-open {
  flex: 1;
  min-width: 0;
  border: 0;
  background: transparent;
  text-align: left;
  padding: 10px 8px;
  border-radius: 10px;
  color: var(--text);
  cursor: pointer;
}
.agent-history-open.current {
  background: var(--accent-soft);
  color: var(--accent-text);
}
.agent-history-open strong {
  display: block;
  font-size: 13px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.agent-history-open small {
  color: var(--text-tertiary);
  font-size: 11px;
}
.agent-thread {
  flex: 1;
  min-width: 0;
  max-height: 62vh;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding: 4px 2px 12px;
}
.agent-row {
  display: flex;
  gap: 10px;
  align-items: flex-start;
}
.agent-row.user {
  justify-content: flex-end;
}
.agent-bubble {
  max-width: 72%;
  background: var(--surface-sunken);
  border-radius: 14px;
  padding: 10px 14px;
  white-space: pre-wrap;
  line-height: 1.55;
}
.agent-avatar {
  width: 28px;
  height: 28px;
  border-radius: 9px;
  background: var(--accent-soft);
  color: var(--accent);
  display: grid;
  place-items: center;
  flex-shrink: 0;
}
.agent-assistant-body {
  flex: 1;
  min-width: 0;
  display: grid;
  gap: 10px;
}
.agent-text {
  white-space: pre-wrap;
  line-height: 1.6;
}
.agent-tool {
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
  background: var(--surface-subtle);
}
.agent-tool header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 9px 12px;
  font-size: 13px;
}
.agent-tool header strong {
  font-weight: 600;
}
.agent-tool-state {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--text-tertiary);
  font-size: 12px;
}
.agent-tool pre {
  margin: 0;
  padding: 10px 12px;
  border-top: 1px solid var(--border);
  font-family: var(--font-mono);
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 220px;
  overflow-y: auto;
}
.agent-tool-result.failed {
  color: var(--danger);
}
.agent-pending {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--text-tertiary);
  font-size: 13px;
}
.agent-composer {
  position: sticky;
  bottom: 0;
  display: flex;
  align-items: flex-end;
  gap: 10px;
  margin-top: 16px;
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: var(--surface);
  box-shadow: 0 8px 24px rgba(15, 23, 42, 0.06);
}
.agent-composer textarea {
  flex: 1;
  border: 0;
  resize: none;
  background: transparent;
  color: var(--text);
  font: inherit;
  line-height: 1.5;
  max-height: 180px;
  min-height: 24px;
  padding: 4px 2px;
}
.agent-composer textarea:focus {
  outline: none;
}
.agent-send {
  white-space: nowrap;
}
.agent-send.stop {
  color: var(--danger);
}
.agent-hint {
  margin: 8px 2px 0;
  color: var(--text-tertiary);
  font-size: 11px;
  text-align: right;
}
</style>
