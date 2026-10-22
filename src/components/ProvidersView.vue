<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue';
import {
  Plus,
  Pencil,
  Trash2,
  Check,
  Server,
  Eye,
  EyeOff,
  LoaderCircle,
  RefreshCw,
} from 'lucide-vue-next';
import { ask } from '@tauri-apps/plugin-dialog';
import { bridge } from '../bridge';
import type { ModelProtocol, ModelProvider, ModelProviders, ProviderInput } from '../types';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import EmptyState from './ui/EmptyState.vue';
import IconTile from './ui/IconTile.vue';
const protocolLabel: Record<ModelProtocol, string> = {
  'openai-compatible': 'OpenAI 兼容',
  anthropic: 'Anthropic',
  gemini: 'Gemini',
};
const book = ref<ModelProviders>({ defaultId: null, providers: [], legacyKeyAvailable: false });
const loading = ref(true);
const busy = ref(false);
const error = ref('');
const status = ref('');
const editing = ref(false);
const showKey = ref(false);
const clearKey = ref(false);
const draft = ref<ProviderInput>(empty());
const models = ref<string[]>([]);
const modelStatus = ref('');
const fetching = ref(false);
let disposed = false;
function empty(): ProviderInput {
  return {
    id: null,
    name: '',
    baseUrl: '',
    protocol: 'openai-compatible',
    model: '',
    apiKey: '',
    useLegacyKey: false,
  };
}
const current = computed(() => book.value.providers.find((p) => p.id === draft.value.id));
const valid = computed(
  () => draft.value.name.trim() && draft.value.baseUrl.trim() && draft.value.model.trim(),
);
function edit(provider?: ModelProvider) {
  draft.value = provider
    ? {
        id: provider.id,
        name: provider.name,
        baseUrl: provider.baseUrl,
        protocol: provider.protocol,
        model: provider.model,
        apiKey: '',
        useLegacyKey: false,
      }
    : empty();
  editing.value = true;
  showKey.value = false;
  clearKey.value = false;
  error.value = '';
  status.value = '';
  models.value = [];
  modelStatus.value = '';
  // Existing providers already have a stored key, so read the catalogue right away.
  if (provider) void fetchModels();
}
function pickModel(event: Event) {
  const select = event.target as HTMLSelectElement;
  if (select.value) draft.value.model = select.value;
  select.value = '';
}
function cancel() {
  editing.value = false;
  draft.value = empty();
  showKey.value = false;
  clearKey.value = false;
  models.value = [];
  modelStatus.value = '';
}
watch(
  () => [draft.value.baseUrl, draft.value.protocol],
  () => {
    models.value = [];
    modelStatus.value = '';
  },
);
async function fetchModels() {
  if (fetching.value || busy.value) return;
  if (!draft.value.baseUrl.trim()) {
    modelStatus.value = '请先填写接口地址';
    return;
  }
  fetching.value = true;
  modelStatus.value = '正在读取…';
  try {
    models.value = await bridge.listProviderModels({
      id: draft.value.id,
      baseUrl: draft.value.baseUrl.trim(),
      protocol: draft.value.protocol,
      apiKey: clearKey.value ? '' : draft.value.apiKey?.trim() || null,
      useLegacyKey: draft.value.useLegacyKey,
    });
    modelStatus.value = models.value.length ? `已读取 ${models.value.length} 个模型` : '';
  } catch (e) {
    models.value = [];
    modelStatus.value = String(e);
  } finally {
    fetching.value = false;
  }
}
async function reload() {
  const data = await bridge.modelProviders();
  if (!disposed) book.value = data;
}
async function run(action: () => Promise<void>, message: string) {
  if (busy.value) return;
  busy.value = true;
  error.value = '';
  status.value = '';
  try {
    await action();
    await reload();
    status.value = message;
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
async function save() {
  if (!valid.value) return;
  await run(async () => {
    const apiKey = clearKey.value ? '' : draft.value.apiKey?.trim() || null;
    await bridge.saveProvider({ ...draft.value, apiKey });
    cancel();
  }, '配置已保存');
}
function select(provider: ModelProvider) {
  return run(() => bridge.selectProvider(provider.id), '已设为默认');
}
async function remove(provider: ModelProvider) {
  await run(async () => {
    if (
      await ask(`移除“${provider.name}”及其保存的 API Key？`, {
        title: '移除模型服务',
        kind: 'warning',
        okLabel: '移除',
        cancelLabel: '取消',
      })
    ) {
      await bridge.removeProvider(provider.id);
      if (draft.value.id === provider.id) cancel();
    }
  }, '');
}
onMounted(async () => {
  try {
    await reload();
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
});
onBeforeUnmount(() => {
  disposed = true;
  draft.value.apiKey = '';
});
</script>
<template>
  <div>
    <Transition name="drop">
      <p v-if="error" class="notice" role="alert">{{ error }}</p>
    </Transition>
    <Transition name="drop">
      <p v-if="status" class="notice success" role="status">{{ status }}</p>
    </Transition>
    <Transition name="section" mode="out-in">
      <SettingGroup
        v-if="!editing"
        key="list"
        title="模型厂商"
        footnote="配置和密钥保存在本机，密钥不会回显。模型调用将在 Agent 接入后启用。"
      >
        <template #actions>
          <button class="btn small" :disabled="busy || loading" @click="edit()">
            <Plus :size="14" />添加厂商
          </button>
        </template>
        <EmptyState v-if="loading" compact :icon="LoaderCircle" busy title="正在读取配置…" />
        <EmptyState
          v-else-if="!book.providers.length"
          compact
          :icon="Server"
          title="尚未配置模型服务"
          description="添加 OpenAI 兼容、Anthropic 或 Gemini 接口"
        >
          <button class="btn primary small" :disabled="busy" @click="edit()">
            <Plus :size="14" />添加厂商
          </button>
        </EmptyState>
        <TransitionGroup v-else name="list">
          <SettingRow v-for="provider in book.providers" :key="provider.id">
            <template #leading>
              <IconTile :tone="provider.id === book.defaultId ? 'blue' : 'gray'" :size="32">
                <Server :size="16" />
              </IconTile>
            </template>
            <template #title>
              {{ provider.name }}
              <span v-if="provider.id === book.defaultId" class="badge accent">默认</span>
              <span class="badge">{{ protocolLabel[provider.protocol] }}</span>
            </template>
            <template #description>
              {{ provider.model }} · {{ provider.keySet ? '已保存密钥' : '未设置密钥' }} ·
              {{ provider.baseUrl }}
            </template>
            <button
              v-if="provider.id !== book.defaultId"
              class="btn ghost small"
              :disabled="busy"
              @click="select(provider)"
            >
              <Check :size="14" />设为默认
            </button>
            <button
              class="btn ghost icon small"
              :title="'编辑' + provider.name"
              :aria-label="'编辑' + provider.name"
              :disabled="busy"
              @click="edit(provider)"
            >
              <Pencil :size="14" />
            </button>
            <button
              class="btn ghost icon small danger"
              :title="'移除' + provider.name"
              :aria-label="'移除' + provider.name"
              :disabled="busy"
              @click="remove(provider)"
            >
              <Trash2 :size="14" />
            </button>
          </SettingRow>
        </TransitionGroup>
      </SettingGroup>

      <SettingGroup v-else key="form" :title="draft.id ? '编辑厂商' : '添加厂商'">
        <form class="provider-form" @submit.prevent="save">
          <fieldset :disabled="busy">
            <SettingRow title="厂商名称" description="用于在列表中区分不同服务">
              <input
                id="provider-name"
                v-model="draft.name"
                class="text-field"
                placeholder="自定义名称"
                aria-label="厂商名称"
                required
                maxlength="60"
              />
            </SettingRow>
            <SettingRow title="接口协议">
              <select
                id="provider-protocol"
                v-model="draft.protocol"
                class="text-field"
                aria-label="接口协议"
              >
                <option value="openai-compatible">OpenAI 兼容</option>
                <option value="anthropic">Anthropic</option>
                <option value="gemini">Gemini</option>
              </select>
            </SettingRow>
            <SettingRow title="接口地址" description="OpenAI 兼容填到 /v1 为止">
              <input
                id="provider-url"
                v-model="draft.baseUrl"
                class="text-field"
                type="url"
                placeholder="https://your-provider.example/v1"
                aria-label="接口地址"
                required
                maxlength="2048"
                spellcheck="false"
              />
            </SettingRow>
            <SettingRow title="模型名称">
              <template #description>{{ modelStatus || '可从厂商读取模型列表' }}</template>
              <div class="model-field">
                <input
                  id="provider-model"
                  v-model="draft.model"
                  class="text-field"
                  list="provider-model-options"
                  placeholder="填写厂商提供的模型 ID"
                  aria-label="模型名称"
                  required
                  maxlength="256"
                  spellcheck="false"
                />
                <select
                  v-if="models.length"
                  class="text-field model-select"
                  aria-label="选择模型"
                  @change="pickModel"
                >
                  <option value="" selected>选择模型…</option>
                  <option v-for="model in models" :key="model" :value="model">{{ model }}</option>
                </select>
                <button type="button" class="btn small" :disabled="fetching" @click="fetchModels">
                  <LoaderCircle v-if="fetching" :size="14" class="spinning" />
                  <RefreshCw v-else :size="14" />
                  获取列表
                </button>
              </div>
            </SettingRow>
            <SettingRow title="API Key">
              <template #description>
                {{ current?.keySet ? '已保存密钥，留空保持不变' : '本地服务可留空' }}
              </template>
              <div class="field-shell">
                <input
                  id="provider-key"
                  v-model="draft.apiKey"
                  :type="showKey ? 'text' : 'password'"
                  :placeholder="current?.keySet ? '留空保留现有密钥' : '输入 API Key'"
                  aria-label="API Key"
                  autocomplete="off"
                  spellcheck="false"
                  maxlength="8192"
                  :disabled="clearKey || draft.useLegacyKey"
                /><button
                  class="btn ghost icon small"
                  type="button"
                  :aria-label="showKey ? '隐藏密钥' : '显示密钥'"
                  @click="showKey = !showKey"
                >
                  <EyeOff v-if="showKey" :size="14" /><Eye v-else :size="14" />
                </button>
              </div>
            </SettingRow>
            <SettingRow v-if="current?.keySet" title="移除已保存的密钥">
              <input
                v-model="clearKey"
                type="checkbox"
                aria-label="移除已保存的密钥"
                :disabled="draft.useLegacyKey"
                @change="draft.apiKey = ''"
              />
            </SettingRow>
            <SettingRow v-if="book.legacyKeyAvailable" title="使用之前保存的 API Key">
              <input
                v-model="draft.useLegacyKey"
                type="checkbox"
                aria-label="使用之前保存的 API Key"
                :disabled="clearKey"
                @change="draft.apiKey = ''"
              />
            </SettingRow>
          </fieldset>
          <div class="form-actions">
            <button type="button" class="btn" @click="cancel">取消</button>
            <button class="btn primary" :disabled="!valid || busy">
              {{ busy ? '保存中…' : '保存配置' }}
            </button>
          </div>
        </form>
      </SettingGroup>
    </Transition>
  </div>
</template>
