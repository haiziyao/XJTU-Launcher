<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { ask } from '@tauri-apps/plugin-dialog';
import { LoaderCircle, Plus, Trash2 } from 'lucide-vue-next';
import { bridge } from '../bridge';
import type { AgentPresetInput, AgentPresetKind, AgentPresets } from '../types';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import EmptyState from './ui/EmptyState.vue';

import UiButton from './ui/UiButton.vue';
import UiIconButton from './ui/UiIconButton.vue';
const props = defineProps<{ kind: AgentPresetKind }>();

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
const status = ref('');
const draft = ref<DraftBase | null>(null);

const titles = {
  role: {
    group: '角色',
    add: '添加角色',
    name: '角色名称',
    note: '角色会作为系统提示词附加到每段对话。',
    foot: '角色在 Agent 页面选择，可随时切换。',
  },
  skill: {
    group: 'Skills',
    add: '添加 Skill',
    name: 'Skill 名称',
    note: 'Skill 由 Agent 按需调用，也可以在输入框用 / 手动指定。',
    foot: 'Skill 指令会在模型调用时加载，不会常驻上下文。',
  },
  task: {
    group: '预设任务',
    add: '添加预设任务',
    name: '任务名称',
    note: '预设任务会填入输入框，发送前还可以修改。',
    foot: '预设任务可以绑定角色与 Skills。',
  },
} as const;
const copy = computed(() => titles[props.kind]);
const list = computed(() => {
  if (props.kind === 'role') return presets.value.roles;
  if (props.kind === 'skill') return presets.value.skills;
  return presets.value.tasks;
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
function edit(item?: { id: string; name: string; description: string } & Record<string, unknown>) {
  draft.value = {
    id: item?.id ?? null,
    name: item?.name ?? '',
    description: item?.description ?? '',
    instructions: (item?.instructions as string) ?? '',
    prompt: (item?.prompt as string) ?? '',
    roleId: (item?.roleId as string) ?? '',
    skillIds: [...((item?.skillIds as string[]) ?? [])],
  };
  status.value = '';
  error.value = '';
}
const valid = computed(() => {
  const value = draft.value;
  if (!value || !value.name.trim()) return false;
  if (props.kind === 'task') return !!value.prompt.trim();
  return !!value.instructions.trim();
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
    status.value = value.id ? '已更新' : '已添加';
    draft.value = null;
    await load();
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
async function remove(item: { id: string; name: string }) {
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
  <div class="preset-panel">
    <Transition name="drop">
      <p v-if="error" class="notice" role="alert">{{ error }}</p>
    </Transition>
    <Transition name="drop">
      <p v-if="status" class="notice success" role="status">{{ status }}</p>
    </Transition>

    <SettingGroup v-if="!draft" :title="copy.group" :footnote="copy.foot">
      <template #actions>
        <UiButton size="sm" :disabled="busy" @click="edit()">
          <Plus :size="14" />{{ copy.add }}
        </UiButton>
      </template>
      <EmptyState
        v-if="loading && !list.length"
        compact
        :icon="LoaderCircle"
        busy
        title="正在读取…"
      />
      <EmptyState
        v-else-if="!list.length"
        compact
        :icon="Plus"
        :title="`还没有${copy.group}`"
        :description="copy.note"
      />
      <SettingRow
        v-for="item in list"
        :key="item.id"
        :title="item.name"
        :description="item.description || ('prompt' in item ? item.prompt : item.instructions)"
      >
        <UiButton size="sm" :disabled="busy" @click="edit(item)">编辑</UiButton>
        <UiIconButton :label="'删除 ' + item.name" :disabled="busy" @click="remove(item)">
          <Trash2 :size="15" />
        </UiIconButton>
      </SettingRow>
    </SettingGroup>

    <form v-else class="preset-form" @submit.prevent="save">
      <h3>{{ draft.id ? '编辑' : copy.add }}</h3>
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
        <label class="preset-field">
          <span>绑定角色</span>
          <select v-model="draft.roleId">
            <option value="">通用角色</option>
            <option v-for="role in presets.roles" :key="role.id" :value="role.id">
              {{ role.name }}
            </option>
          </select>
        </label>
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
      <div class="preset-actions">
        <UiButton variant="ghost" type="button" :disabled="busy" @click="draft = null"
          >取消</UiButton
        >
        <UiButton variant="primary" type="submit" :disabled="!valid || busy">
          {{ busy ? '保存中…' : '保存' }}
        </UiButton>
      </div>
    </form>
  </div>
</template>
<style scoped>
.preset-form {
  display: grid;
  gap: 14px;
  padding: 18px 14px;
  border: 1px solid var(--border);
  border-radius: 14px;
  margin-bottom: 24px;
}
.preset-form h3 {
  margin: 0;
  font-size: 15px;
}
.preset-field {
  display: grid;
  gap: 6px;
  font-size: 12px;
  color: var(--text-tertiary);
}
.preset-field input,
.preset-field textarea,
.preset-field select {
  border: 1px solid var(--control-border);
  border-radius: var(--radius);
  background: var(--control-bg);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  padding: 8px 10px;
  resize: vertical;
}
.preset-field select {
  appearance: none;
  padding-right: 30px;
  background-image: var(--chevron-down);
  background-repeat: no-repeat;
  background-position: right 10px center;
}
.preset-field input:focus,
.preset-field textarea:focus,
.preset-field select:focus {
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
.preset-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
