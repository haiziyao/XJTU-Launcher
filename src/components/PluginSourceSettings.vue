<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { ask } from '@tauri-apps/plugin-dialog';
import { LockKeyhole } from 'lucide-vue-next';
import { bridge } from '../bridge';
import type { AppConfig } from '../types';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import UiSwitch from './ui/UiSwitch.vue';
const props = defineProps<{
  config: AppConfig;
  saving: boolean;
  saveSource: (source: string | null) => Promise<boolean>;
}>();
const emit = defineEmits<{ update: [config: AppConfig] }>();
const source = ref<string | null>(null);
const editing = ref(false);
const busy = ref(false);
const draft = ref('');
const error = ref('');
async function load() {
  const result = await bridge.pluginSource();
  source.value = result.source;
}
onMounted(() => {
  void load().catch((e) => (error.value = String(e)));
});
async function unlock() {
  busy.value = true;
  try {
    if (
      await ask('更改后，插件列表、下载和更新将来自新地址。不可信的服务端可能提供有害插件。', {
        title: '更改插件服务端',
        kind: 'warning',
        okLabel: '了解风险，继续',
        cancelLabel: '取消',
      })
    ) {
      draft.value = source.value || '';
      editing.value = true;
      error.value = '';
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
async function save(value: string | null) {
  busy.value = true;
  error.value = '';
  try {
    if (!(await props.saveSource(value))) throw new Error('地址未保存，请检查格式后重试');
    await load();
    editing.value = false;
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <div class="source-settings">
    <p v-if="error" class="notice" role="alert">{{ error }}</p>
    <SettingGroup title="插件服务端">
      <SettingRow title="服务端网址">
        <template v-if="!editing"
          ><span class="source-address"><LockKeyhole :size="14" />{{ source || '尚未配置' }}</span
          ><button class="btn small" :disabled="busy || saving" @click="unlock">
            确认更改
          </button></template
        >
        <form v-else class="source-form" @submit.prevent="save(draft.trim())">
          <input
            v-model="draft"
            autofocus
            type="url"
            required
            aria-label="插件服务端网址"
            placeholder="https://"
            :disabled="busy"
          /><button type="submit" class="btn primary small" :disabled="busy || saving">保存</button
          ><button
            type="button"
            class="btn small"
            :disabled="busy"
            @click="
              editing = false;
              error = '';
            "
          >
            取消
          </button>
        </form>
      </SettingRow>
      <SettingRow title="恢复默认地址"
        ><button
          class="btn small"
          :disabled="busy || saving || !config.pluginServer"
          @click="save(null)"
        >
          恢复默认
        </button></SettingRow
      >
    </SettingGroup>
    <SettingGroup title="插件更新"
      ><SettingRow title="自动检查插件更新"
        ><UiSwitch
          :checked="config.checkPluginUpdates"
          label="自动检查插件更新"
          :disabled="saving"
          @toggle="
            emit('update', { ...config, checkPluginUpdates: !config.checkPluginUpdates })
          " /></SettingRow
    ></SettingGroup>
  </div>
</template>
<style scoped>
.source-address {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-secondary);
  max-width: 350px;
  overflow-wrap: anywhere;
}
.source-form {
  display: flex;
  gap: 8px;
  align-items: center;
}
.source-form input {
  width: min(280px, 28vw);
}
</style>
