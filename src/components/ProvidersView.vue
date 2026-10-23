<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue';
import {
  Plus,
  Pencil,
  Trash2,
  Check,
  Play,
  PlugZap,
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
import MenuSelect from './ui/MenuSelect.vue';
import EmptyState from './ui/EmptyState.vue';
import IconTile from './ui/IconTile.vue';
import UiButton from './ui/UiButton.vue';
import UiIconButton from './ui/UiIconButton.vue';
const protocolLabel: Record<ModelProtocol, string> = {
  'openai-compatible': 'OpenAI 兼容',
  anthropic: 'Anthropic',
  gemini: 'Gemini',
};
const protocolOptions = [
  { value: 'openai-compatible', label: 'OpenAI 兼容' },
  { value: 'anthropic', label: 'Anthropic' },
  { value: 'gemini', label: 'Gemini' },
];
function setProtocol(value: string) {
  draft.value.protocol = value as ModelProtocol;
}
const book = ref<ModelProviders>({ defaultId: null, providers: [], legacyKeyAvailable: false });
const loading = ref(true);
const busy = ref(false);
const error = ref('');
const status = ref('');
const testing = ref('');
const editing = ref(false);
const showKey = ref(false);
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
    models: [],
    apiKey: '',
    useLegacyKey: false,
  };
}
const current = computed(() => book.value.providers.find((p) => p.id === draft.value.id));
const valid = computed(() => draft.value.name.trim() && draft.value.baseUrl.trim());
function edit(provider?: ModelProvider) {
  draft.value = provider
    ? {
        id: provider.id,
        name: provider.name,
        baseUrl: provider.baseUrl,
        protocol: provider.protocol,
        model: provider.model,
        models: provider.models,
        apiKey: '',
        useLegacyKey: false,
      }
    : empty();
  editing.value = true;
  showKey.value = false;
  error.value = '';
  status.value = '';
  models.value = provider?.models.slice() ?? [];
  modelStatus.value = '';
  // Existing providers already have a stored key, so refresh the catalogue right away.
  if (provider) void fetchModels();
}
function cancel() {
  editing.value = false;
  draft.value = empty();
  showKey.value = false;
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
/** Reveals the stored key on demand; the field stays empty until then. */
async function toggleKey() {
  if (!showKey.value && !draft.value.apiKey && current.value?.keySet && draft.value.id) {
    try {
      draft.value.apiKey = await bridge.revealProviderKey(draft.value.id);
    } catch (e) {
      error.value = String(e);
      return;
    }
  }
  showKey.value = !showKey.value;
}
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
      apiKey: draft.value.apiKey?.trim() || null,
      useLegacyKey: draft.value.useLegacyKey,
    });
    modelStatus.value = models.value.length ? `已获取 ${models.value.length} 个模型` : '';
    draft.value.models = models.value;
    // A saved provider shares the catalogue with the chat page immediately.
    if (draft.value.id) await bridge.saveProviderModels(draft.value.id, models.value);
    // The chat page owns the real choice; this only supplies a starting point.
    if (!draft.value.model && models.value.length) draft.value.model = models.value[0];
  } catch (e) {
    models.value = [];
    modelStatus.value = String(e);
  } finally {
    fetching.value = false;
  }
}
async function test(provider: ModelProvider) {
  if (testing.value) return;
  testing.value = provider.id;
  status.value = '';
  error.value = '';
  try {
    const count = await bridge.testProvider(provider.id);
    status.value = `${provider.name} 连接正常${count ? `，可获取 ${count} 个模型` : ''}`;
  } catch (e) {
    error.value = `${provider.name} 连接失败：${String(e)}`;
  } finally {
    testing.value = '';
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
    const apiKey = draft.value.apiKey?.trim() || null;
    await bridge.saveProvider({ ...draft.value, models: models.value, apiKey });
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
        footnote="配置和密钥保存在本机；密钥默认隐藏，可点右侧按钮查看或替换。"
      >
        <template #actions>
          <UiButton size="sm" :disabled="busy || loading" @click="edit()">
            <Plus :size="14" />添加厂商
          </UiButton>
        </template>
        <EmptyState v-if="loading" compact :icon="LoaderCircle" busy title="正在读取配置…" />
        <EmptyState
          v-else-if="!book.providers.length"
          compact
          :icon="Server"
          title="尚未配置模型服务"
          description="添加 OpenAI 兼容、Anthropic 或 Gemini 接口"
        >
          <UiButton variant="primary" size="sm" :disabled="busy" @click="edit()">
            <Plus :size="14" />添加厂商
          </UiButton>
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
            <UiIconButton
              :label="provider.id === book.defaultId ? '正在使用' : `使用${provider.name}`"
              size="sm"
              :class="{ accent: provider.id === book.defaultId }"
              :title="provider.id === book.defaultId ? '正在使用' : `使用${provider.name}`"
              :disabled="busy || provider.id === book.defaultId"
              :aria-pressed="provider.id === book.defaultId"
              @click="select(provider)"
            >
              <Check v-if="provider.id === book.defaultId" :size="14" />
              <Play v-else :size="14" />
            </UiIconButton>
            <UiIconButton
              :label="`测试${provider.name}连通性`"
              size="sm"
              :title="`测试${provider.name}连通性`"
              :disabled="busy || testing === provider.id"
              @click="test(provider)"
            >
              <LoaderCircle v-if="testing === provider.id" :size="14" class="spinning" />
              <PlugZap v-else :size="14" />
            </UiIconButton>
            <UiIconButton
              :label="'编辑' + provider.name"
              size="sm"
              :title="'编辑' + provider.name"
              :disabled="busy"
              @click="edit(provider)"
            >
              <Pencil :size="14" />
            </UiIconButton>
            <UiIconButton
              :label="'移除' + provider.name"
              variant="danger"
              size="sm"
              :title="'移除' + provider.name"
              :disabled="busy"
              @click="remove(provider)"
            >
              <Trash2 :size="14" />
            </UiIconButton>
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
              <MenuSelect
                :model-value="draft.protocol"
                :options="protocolOptions"
                label="接口协议"
                @update:model-value="setProtocol"
              />
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
            <SettingRow title="模型列表">
              <template #description>
                {{
                  modelStatus ||
                  '获取厂商提供的模型，用于展示；具体使用哪个模型在 Agent 对话框里选择'
                }}
              </template>
              <UiButton size="sm" type="button" :disabled="fetching || busy" @click="fetchModels">
                <LoaderCircle v-if="fetching" :size="14" class="spinning" />
                <RefreshCw v-else :size="14" />
                获取模型列表
              </UiButton>
            </SettingRow>
            <SettingRow v-if="models.length" title="已获取的模型">
              <div class="model-cloud">
                <span v-for="model in models" :key="model" class="badge">{{ model }}</span>
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
                  :placeholder="
                    current?.keySet ? '已保存，点击右侧按钮查看或直接输入新密钥' : '输入 API Key'
                  "
                  aria-label="API Key"
                  autocomplete="off"
                  spellcheck="false"
                  maxlength="8192"
                  :disabled="draft.useLegacyKey"
                /><UiIconButton
                  :label="showKey ? '隐藏密钥' : '显示密钥'"
                  size="sm"
                  type="button"
                  @click="toggleKey"
                >
                  <EyeOff v-if="showKey" :size="14" /><Eye v-else :size="14" />
                </UiIconButton>
              </div>
            </SettingRow>
            <SettingRow v-if="book.legacyKeyAvailable" title="使用之前保存的 API Key">
              <input
                v-model="draft.useLegacyKey"
                type="checkbox"
                aria-label="使用之前保存的 API Key"
                @change="draft.apiKey = ''"
              />
            </SettingRow>
          </fieldset>
          <div class="form-actions">
            <UiButton type="button" @click="cancel">取消</UiButton>
            <UiButton variant="primary" :disabled="!valid || busy">
              {{ busy ? '保存中…' : '保存配置' }}
            </UiButton>
          </div>
        </form>
      </SettingGroup>
    </Transition>
  </div>
</template>
<style scoped>
.ui-icon-button.accent {
  color: var(--accent);
}
.ui-icon-button.accent:disabled {
  opacity: 1;
}
.model-cloud {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  justify-content: flex-end;
  max-width: 60%;
}
</style>
