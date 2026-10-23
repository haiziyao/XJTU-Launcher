<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Download } from 'lucide-vue-next';
import { bridge } from '../bridge';
import { initializeUpdates, updateStatus } from '../updates';
import type { AppConfig } from '../types';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import UiSwitch from './ui/UiSwitch.vue';
import IconTile from './ui/IconTile.vue';
import UiButton from './ui/UiButton.vue';
const props = defineProps<{ config: AppConfig; saving: boolean }>();
const emit = defineEmits<{ update: [config: AppConfig] }>();
const error = ref('');
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
        <p>{{ updateStatus.notes || '此版本未提供更新说明。' }}</p>
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
.update-release p {
  white-space: pre-wrap;
  color: var(--text-secondary);
  margin: 18px 0;
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
