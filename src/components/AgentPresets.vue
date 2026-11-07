<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { ask } from '@tauri-apps/plugin-dialog';
import { IconLoader2, IconPlus, IconTrash } from '@tabler/icons-vue';
import { bridge } from '../bridge';
import type {
  AgentPresetInput,
  AgentPresetKind,
  AgentPresets,
  AgentRole,
  AgentTask,
} from '../types';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import EmptyState from './ui/EmptyState.vue';
import IconTile from './ui/IconTile.vue';
import SettingsDialog from './ui/SettingsDialog.vue';

import UiButton from './ui/UiButton.vue';
import MenuSelect from './ui/MenuSelect.vue';
const props = defineProps<{ kind: AgentPresetKind }>();

type Preset = AgentRole | AgentTask;

interface DraftBase {
  id: string | null;
  name: string;
  description: string;
  instructions: string;
  prompt: string;
  roleId: string;
  skillIds: string[];
}

const presets = ref<AgentPresets>({ version: 1, roles: [], skills: [], tasks: [] });
const loading = ref(true);
const busy = ref(false);
const error = ref('');
const draft = ref<DraftBase | null>(null);
const viewed = ref<Preset | null>(null);
const roleOptions = computed(() => [
  { value: '', label: '通用角色' },
  ...presets.value.roles.map((role) => ({ value: role.id, label: role.name })),
]);
const skillNames = computed(() =>
  Object.fromEntries(presets.value.skills.map((skill) => [skill.id, skill.name])),
);
const viewedRole = computed(() => {
  const item = viewed.value;
  if (!item || !('roleId' in item)) return '';
  return roleOptions.value.find((option) => option.value === item.roleId)?.label || '通用角色';
});
const viewedSkills = computed(() => {
  const item = viewed.value;
  if (!item || !('skillIds' in item)) return '';
  return (item.skillIds ?? [])
    .map((id) => skillNames.value[id])
    .filter(Boolean)
    .join('、');
});
function setRole(value: string) {
  if (draft.value) draft.value.roleId = value;
}

const titles = {
  role: {
    group: '角色',
    add: '新增角色',
    name: '角色名称',
    note: '角色会作为系统提示词附加到每段对话。',
  },
  skill: {
    group: 'Skills',
    add: '新增 Skill',
    name: 'Skill 名称',
    note: 'Skill 由 Agent 按需调用，也可以在输入框用 / 手动指定。',
  },
  task: {
    group: '预设任务',
    add: '新增预设任务',
    name: '任务名称',
    note: '预设任务会填入输入框，发送前还可以修改。',
  },
} as const;
const copy = computed(() => titles[props.kind]);
const list = computed<Preset[]>(() => {
  if (props.kind === 'role') return presets.value.roles;
  if (props.kind === 'skill') return presets.value.skills;
  return presets.value.tasks;
});
const dialogOpen = computed(() => viewed.value !== null || draft.value !== null);
const dialogTitle = computed(() => {
  if (draft.value) return draft.value.id ? `编辑${copy.value.group}` : copy.value.add;
  return viewed.value?.name ?? '';
});

async function load() {
  loading.value = true;
  try {
    presets.value = await bridge.agentPresets();
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}
function openView(item: Preset) {
  error.value = '';
  viewed.value = item;
  draft.value = null;
}
function openNew() {
  error.value = '';
  viewed.value = null;
  draft.value = {
    id: null,
    name: '',
    description: '',
    instructions: '',
    prompt: '',
    roleId: '',
    skillIds: [],
  };
}
function openEdit(item: Preset) {
  error.value = '';
  viewed.value = null;
  draft.value = {
    id: item.id,
    name: item.name,
    description: item.description ?? '',
    instructions: ('instructions' in item ? item.instructions : '') ?? '',
    prompt: ('prompt' in item ? item.prompt : '') ?? '',
    roleId: ('roleId' in item ? item.roleId : '') ?? '',
    skillIds: [...('skillIds' in item ? (item.skillIds ?? []) : [])],
  };
}
function close() {
  viewed.value = null;
  draft.value = null;
  error.value = '';
}
const valid = computed(() => {
  const value = draft.value;
  if (!value || !value.name.trim()) return false;
  if (props.kind === 'task') return !!value.prompt.trim();
  return !!value.instructions.trim();
});
/** Names the empty field so a disabled save button is not a silent dead end. */
const missing = computed(() => {
  const value = draft.value;
  if (!value) return '';
  if (!value.name.trim()) return copy.value.name;
  if (props.kind === 'task') return value.prompt.trim() ? '' : '任务内容';
  return value.instructions.trim() ? '' : '指令';
});
async function save() {
  const value = draft.value;
  if (!value || !valid.value || busy.value) return;
  busy.value = true;
  error.value = '';
  try {
    const input: AgentPresetInput =
      props.kind === 'task'
        ? {
            kind: 'task',
            id: value.id,
            name: value.name.trim(),
            description: value.description.trim(),
            prompt: value.prompt,
            roleId: value.roleId || null,
            skillIds: value.skillIds,
          }
        : {
            kind: props.kind,
            id: value.id,
            name: value.name.trim(),
            description: value.description.trim(),
            instructions: value.instructions,
          };
    await bridge.agentSavePreset(input);
    close();
    await load();
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
async function remove(item: Preset) {
  if (
    !(await ask(`删除“${item.name}”？`, {
      title: `删除${copy.value.group}`,
      kind: 'warning',
      okLabel: '删除',
      cancelLabel: '取消',
    }))
  )
    return;
  busy.value = true;
  error.value = '';
  try {
    await bridge.agentRemovePreset(props.kind, item.id);
    close();
    await load();
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
function toggleSkill(id: string) {
  const value = draft.value;
  if (!value) return;
  value.skillIds = value.skillIds.includes(id)
    ? value.skillIds.filter((item) => item !== id)
    : [...value.skillIds, id];
}
onMounted(() => void load());
</script>
<template>
  <Transition name="drop">
    <p v-if="error && !dialogOpen" class="notice" role="alert">{{ error }}</p>
  </Transition>
  <SettingGroup :title="copy.group">
    <EmptyState
      v-if="loading && !list.length"
      compact
      :icon="IconLoader2"
      busy
      title="正在读取…"
    />
    <SettingRow
      v-for="item in list"
      :key="item.id"
      clickable
      :title="item.name"
      :disabled="busy"
      @select="openView(item)"
    >
    </SettingRow>
    <SettingRow clickable :title="copy.add" :disabled="busy" @select="openNew">
      <template #leading
        ><IconTile :size="30"><IconPlus :size="16" /></IconTile
      ></template>
    </SettingRow>
  </SettingGroup>

  <SettingsDialog :open="dialogOpen" :title="dialogTitle" wide @close="close">
    <Transition name="drop">
      <p v-if="error" class="notice" role="alert">{{ error }}</p>
    </Transition>

    <template v-if="viewed">
      <div v-if="viewed.description" class="preset-detail">{{ viewed.description }}</div>
      <div class="preset-detail">
        <span>{{ 'prompt' in viewed ? '任务内容' : '指令' }}</span>
        <pre>{{ 'prompt' in viewed ? viewed.prompt : viewed.instructions }}</pre>
      </div>
      <div v-if="'roleId' in viewed" class="preset-detail">
        <span>绑定角色</span>
        <p>{{ viewedRole }}</p>
      </div>
      <div v-if="'skillIds' in viewed && viewed.skillIds?.length" class="preset-detail">
        <span>绑定 Skills</span>
        <p>{{ viewedSkills }}</p>
      </div>
    </template>

    <form v-else-if="draft" class="preset-form" @submit.prevent="save">
      <label class="preset-field">
        <span>{{ copy.name }}</span>
        <input v-model="draft.name" maxlength="40" required />
      </label>
      <label class="preset-field">
        <span>说明（可选）</span>
        <input v-model="draft.description" maxlength="200" />
      </label>
      <label v-if="kind !== 'task'" class="preset-field">
        <span>指令</span>
        <textarea v-model="draft.instructions" rows="7" :placeholder="copy.note"></textarea>
      </label>
      <template v-else>
        <label class="preset-field">
          <span>任务内容</span>
          <textarea
            v-model="draft.prompt"
            rows="4"
            placeholder="例如：整理本周课表和作业安排"
          ></textarea>
        </label>
        <div class="preset-field">
          <span>绑定角色</span>
          <MenuSelect
            class="preset-select"
            :model-value="draft.roleId"
            :options="roleOptions"
            label="绑定角色"
            @update:model-value="setRole"
          />
        </div>
        <div v-if="presets.skills.length" class="preset-field">
          <span>绑定 Skills</span>
          <div class="preset-skills">
            <label v-for="skill in presets.skills" :key="skill.id">
              <input
                type="checkbox"
                :checked="draft.skillIds.includes(skill.id)"
                @change="toggleSkill(skill.id)"
              />
              {{ skill.name }}
            </label>
          </div>
        </div>
      </template>
      <p v-if="missing" class="preset-hint" aria-live="polite">请填写{{ missing }}后再保存。</p>
    </form>

    <template #footer>
      <template v-if="viewed">
        <UiButton variant="ghost" size="sm" danger :disabled="busy" @click="remove(viewed)">
          <IconTrash :size="13" />删除
        </UiButton>
        <UiButton variant="primary" size="sm" :disabled="busy" @click="openEdit(viewed)"
          >编辑</UiButton
        >
      </template>
      <template v-else>
        <UiButton variant="ghost" size="sm" :disabled="busy" @click="close">取消</UiButton>
        <UiButton variant="primary" size="sm" :disabled="!valid || busy" @click="save">
          {{ busy ? '保存中…' : '保存' }}
        </UiButton>
      </template>
    </template>
  </SettingsDialog>
</template>
<style scoped>
.preset-detail {
  display: grid;
  gap: 6px;
  font-size: 12px;
  color: var(--text-tertiary);
}
.preset-detail p,
.preset-detail pre {
  margin: 0;
  color: var(--text);
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.preset-form {
  display: grid;
  gap: 14px;
}
.preset-field {
  display: grid;
  gap: 6px;
  font-size: 12px;
  color: var(--text-tertiary);
}
.preset-hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-tertiary);
}
.preset-field input,
.preset-field textarea {
  border: 1px solid var(--control-border);
  border-radius: var(--radius);
  background: var(--control-bg);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  padding: 8px 10px;
  resize: vertical;
}
/* Match the textareas beside it; MenuSelect draws its own shell. */
.preset-field .preset-select :deep(.field-shell) {
  height: 36px;
  padding: 0 6px 0 10px;
  border-radius: var(--radius);
}
.preset-field .preset-select :deep(.menu-field-value) {
  font-size: 13px;
}
.preset-field input:focus,
.preset-field textarea:focus {
  border-color: var(--accent);
  box-shadow: var(--focus-ring);
  outline: none;
}
.preset-skills {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  padding: 4px 0;
}
.preset-skills label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--text);
}
</style>
