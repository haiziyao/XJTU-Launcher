<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { Download } from 'lucide-vue-next';
import { bridge } from '../bridge';
import { initializeUpdates, updateStatus } from '../updates';
import { renderMarkdown } from '../markdown';
import type { AppConfig } from '../types';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import UiSwitch from './ui/UiSwitch.vue';
import IconTile from './ui/IconTile.vue';
import UiButton from './ui/UiButton.vue';
const props = defineProps<{ config: AppConfig; saving: boolean }>();
const emit = defineEmits<{ update: [config: AppConfig] }>();
const error = ref('');
const endpoint = ref(props.config.updateEndpoint);
const endpointValid = computed(() => {
  const value = endpoint.value.trim();
  if (!value) return true;
  try {
    const url = new URL(value);
    return (
      ['http:', 'https:'].includes(url.protocol) && !url.username && !url.password && !!url.hostname
    );
  } catch {
    return false;
  }
});
const notes = computed(() =>
  updateStatus.value?.notes
    ? renderMarkdown(updateStatus.value.notes)
    : '<p>此版本未提供更新说明。</p>',
);
watch(
  () => props.config.updateEndpoint,
  (value) => {
    endpoint.value = value;
  },
);
function saveEndpoint() {
  if (!endpointValid.value) return;
  emit('update', { ...props.config, updateEndpoint: endpoint.value.trim() });
}
function resetEndpoint() {
  endpoint.value = '';
  emit('update', { ...props.config, updateEndpoint: '' });
}
function noteClick(event: MouseEvent) {
  const anchor = (event.target as Element | null)?.closest('a');
  if (!(anchor instanceof HTMLAnchorElement)) return;
  event.preventDefault();
  void bridge.openUrl(anchor.href);
}
const busy = computed(
  () =>
    !updateStatus.value ||
    ['checking', 'downloading', 'installing'].includes(updateStatus.value.phase),
);
const percent = computed(() =>
  updateStatus.value?.total
    ? Math.min(100, Math.round((updateStatus.value.received / updateStatus.value.total) * 100))
    : undefined,
);
onMounted(() => {
  void initializeUpdates(props.config.checkAppUpdates).catch((e) => (error.value = String(e)));
});
async function check() {
  error.value = '';
  try {
    updateStatus.value = await bridge.checkUpdate();
  } catch (e) {
    error.value = String(e);
  }
}
async function install() {
  error.value = '';
  try {
    await bridge.installUpdate();
  } catch (e) {
    error.value = String(e);
  }
}
async function restart() {
  try {
    await bridge.restartUpdatedApp();
  } catch (e) {
    error.value = String(e);
  }
}
</script>
<template>
  <div class="update-settings">
    <SettingGroup>
      <SettingRow
        title="XJTU Launcher"
        :description="updateStatus ? `当前版本 ${updateStatus.currentVersion}` : '正在读取版本…'"
      >
        <template #leading
          ><IconTile :size="42"><Download :size="24" /></IconTile
        ></template>
        <UiButton
          size="sm"
          :disabled="
            busy || updateStatus?.phase === 'installed' || updateStatus?.phase === 'unconfigured'
          "
          @click="check"
        >
          {{ updateStatus?.phase === 'checking' ? '正在检查…' : '检查更新' }}
        </UiButton>
      </SettingRow>
      <SettingRow title="自动检查更新"
        ><UiSwitch
          :checked="config.checkAppUpdates"
          label="自动检查更新"
          :disabled="saving"
          @toggle="emit('update', { ...config, checkAppUpdates: !config.checkAppUpdates })"
      /></SettingRow>
    </SettingGroup>
    <SettingGroup title="更新源">
      <SettingRow
        title="镜像地址"
        description="GitHub 无法访问时填写镜像的 latest.json 地址，留空使用内置地址。镜像需同时提供安装包，并把清单里的下载地址指向镜像。"
      >
        <div class="update-source">
          <input
            v-model="endpoint"
            class="text-field"
            placeholder="https://…/latest.json"
            spellcheck="false"
            autocapitalize="none"
          />
          <UiButton size="sm" :disabled="saving || !endpointValid" @click="saveEndpoint"
            >保存</UiButton
          >
          <UiButton v-if="config.updateEndpoint" size="sm" variant="ghost" @click="resetEndpoint"
            >恢复默认</UiButton
          >
        </div>
      </SettingRow>
      <p v-if="!endpointValid" class="update-hint">请输入有效地址，且不要包含账号密码。</p>
    </SettingGroup>
    <p v-if="error || updateStatus?.message" class="notice" role="alert">
      {{ error || updateStatus?.message }}
    </p>
    <p v-if="updateStatus?.phase === 'unconfigured'" class="update-hint">
      此构建尚未配置更新服务。
    </p>
    <p v-else-if="updateStatus?.phase === 'current'" class="update-hint" role="status">
      当前已是最新版本。
    </p>
    <SettingGroup
      v-if="updateStatus?.version"
      :title="updateStatus.phase === 'installed' ? '更新已安装' : '发现新版本'"
    >
      <div class="update-release">
        <strong>{{ updateStatus.version }}</strong>
        <div class="update-notes" @click="noteClick" v-html="notes"></div>
        <template v-if="['downloading', 'installing'].includes(updateStatus.phase)"
          ><progress :value="percent" max="100" aria-label="更新下载进度" />
          <p role="status">
            {{
              updateStatus.phase === 'installing'
                ? '正在安装…'
                : percent === undefined
                  ? '正在下载…'
                  : `正在下载 ${percent}%`
            }}
          </p></template
        >
        <div class="update-action">
          <UiButton variant="primary" v-if="updateStatus.phase === 'installed'" @click="restart">
            重新启动</UiButton
          ><UiButton variant="primary" v-else :disabled="busy" @click="install"
            >下载并更新</UiButton
          >
        </div>
      </div>
    </SettingGroup>
  </div>
</template>
<style scoped>
.update-hint {
  color: var(--text-secondary);
  padding: 12px;
}
.update-release {
  padding: 18px 12px;
}
.update-notes {
  max-height: 320px;
  overflow-y: auto;
  margin: 16px 0;
  color: var(--text-secondary);
  font-size: 12.5px;
  line-height: 1.7;
}
.update-notes :deep(h1),
.update-notes :deep(h2),
.update-notes :deep(h3) {
  margin: 14px 0 6px;
  color: var(--text);
  font-size: 13px;
  font-weight: 600;
}
.update-notes :deep(h1:first-child),
.update-notes :deep(h2:first-child),
.update-notes :deep(h3:first-child) {
  margin-top: 0;
}
.update-notes :deep(p) {
  margin: 8px 0;
}
.update-notes :deep(ul),
.update-notes :deep(ol) {
  margin: 8px 0;
  padding-left: 18px;
}
.update-notes :deep(li) {
  margin: 3px 0;
}
.update-notes :deep(code) {
  padding: 1px 5px;
  border-radius: var(--radius-xs);
  background: var(--surface-sunken);
  font-family: var(--font-mono);
  font-size: 11.5px;
}
.update-notes :deep(a) {
  color: var(--accent-text);
}
.update-source {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  max-width: 560px;
}
.update-source .text-field {
  flex: 1;
  min-width: 0;
}
.update-action {
  display: flex;
  justify-content: flex-end;
  margin-top: 28px;
}
progress {
  width: 100%;
  accent-color: var(--accent);
  height: 5px;
}
</style>
