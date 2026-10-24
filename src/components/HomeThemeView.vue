<script setup lang="ts">
import { computed } from 'vue';
import { ArrowDown, ArrowUp, Check, Clock, LayoutGrid } from 'lucide-vue-next';
import type { AppConfig } from '../types';
import PageHeader from './ui/PageHeader.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import UiSwitch from './ui/UiSwitch.vue';
import UiIconButton from './ui/UiIconButton.vue';

const props = defineProps<{ config: AppConfig; saving: boolean }>();
const emit = defineEmits<{ update: [config: AppConfig] }>();

const CLOCKS = [
  { id: 'plain', name: '简约', hint: '轻量细字' },
  { id: 'bold', name: '醒目', hint: '大号粗体' },
  { id: 'mono', name: '等宽', hint: '等宽数字' },
  { id: 'card', name: '卡片', hint: '浅底圆角' },
];
const SECTIONS = [
  { id: 'greeting', name: '问候与日期', description: '左上角的日期和问候语' },
  { id: 'clock', name: '时钟', description: '右上角的时间显示' },
  { id: 'timetable', name: '今日课程', description: '当天的课程卡片' },
  { id: 'links', name: '校园快捷入口', description: '一网通办、教务、思源学堂、图书馆' },
  { id: 'notices', name: '教务处通知', description: '教务处教学通知，可在主页上下滚动查看' },
  { id: 'plugins', name: '我的插件', description: '已安装插件的快捷入口' },
];
const visible = computed(() => new Set(props.config.homeSections));
function setClock(id: string) {
  if (id !== props.config.homeClock) emit('update', { ...props.config, homeClock: id });
}
function toggleSection(id: string, on: boolean) {
  const next = on
    ? [...props.config.homeSections, id]
    : props.config.homeSections.filter((item) => item !== id);
  emit('update', { ...props.config, homeSections: next });
}
const CARD_SECTIONS = ['timetable', 'links', 'notices', 'plugins'] as const;
const orderedCards = computed(() => {
  const order = props.config.widgets;
  return [...CARD_SECTIONS].sort((left, right) => {
    const a = order.indexOf(left);
    const b = order.indexOf(right);
    return (a < 0 ? Number.MAX_SAFE_INTEGER : a) - (b < 0 ? Number.MAX_SAFE_INTEGER : b);
  });
});
function sectionName(id: string) {
  return SECTIONS.find((section) => section.id === id)?.name ?? id;
}
function moveSection(id: string, delta: number) {
  const list = [...props.config.widgets];
  const index = list.indexOf(id);
  const target = index + delta;
  if (index < 0 || target < 0 || target >= list.length) return;
  [list[index], list[target]] = [list[target], list[index]];
  emit('update', { ...props.config, widgets: list });
}
</script>
<template>
  <section class="desktop-page">
    <PageHeader title="主页主题" description="调整主页的时钟样式与显示的元素" />
    <SettingGroup title="时钟风格">
      <div class="clock-picker" role="radiogroup" aria-label="时钟风格">
        <button
          v-for="clock in CLOCKS"
          :key="clock.id"
          type="button"
          class="clock-option"
          :class="{ active: config.homeClock === clock.id }"
          role="radio"
          :aria-checked="config.homeClock === clock.id"
          :disabled="saving"
          @click="setClock(clock.id)"
        >
          <span class="clock-preview" :class="`clock-${clock.id}`">
            <Clock v-if="clock.id === 'card'" :size="13" />12:34
          </span>
          <strong>{{ clock.name }}</strong>
          <small>{{ clock.hint }}</small>
          <span v-if="config.homeClock === clock.id" class="clock-check"><Check :size="12" /></span>
        </button>
      </div>
    </SettingGroup>
    <SettingGroup title="主页元素">
      <SettingRow
        v-for="section in SECTIONS"
        :key="section.id"
        :title="section.name"
        :description="section.description"
      >
        <UiSwitch
          :checked="visible.has(section.id)"
          :label="section.name"
          :disabled="saving"
          @toggle="toggleSection(section.id, !visible.has(section.id))"
        />
      </SettingRow>
    </SettingGroup>
    <SettingGroup title="显示顺序">
      <SettingRow
        v-for="(id, index) in orderedCards"
        :key="id"
        :title="sectionName(id)"
        :description="visible.has(id) ? '当前显示' : '当前关闭，顺序仍保留'"
      >
        <div class="order-actions">
          <UiIconButton
            label="上移"
            size="sm"
            :disabled="saving || index === 0"
            @click="moveSection(id, -1)"
            ><ArrowUp :size="14"
          /></UiIconButton>
          <UiIconButton
            label="下移"
            size="sm"
            :disabled="saving || index === orderedCards.length - 1"
            @click="moveSection(id, 1)"
            ><ArrowDown :size="14"
          /></UiIconButton>
        </div>
      </SettingRow>
    </SettingGroup>
    <p class="theme-footnote">
      <LayoutGrid :size="13" />关闭全部元素后，主页会显示一个空状态提示。
    </p>
  </section>
</template>
<style scoped>
.order-actions {
  display: flex;
  gap: 6px;
}
.clock-picker {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
  padding: 12px;
}
.clock-option {
  position: relative;
  display: grid;
  justify-items: center;
  gap: 2px;
  padding: 12px 8px 10px;
  border: 1px solid transparent;
  border-radius: var(--radius);
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    border-color var(--duration) ease,
    color var(--duration) ease;
}
.clock-option:hover:not(:disabled):not(.active) {
  background: var(--accent-wash);
}
.clock-option.active {
  border-color: var(--accent-outline);
  background: var(--accent-tint);
  color: var(--accent-text);
}
.clock-option strong {
  margin-top: 6px;
  font-size: 12.5px;
  font-weight: 600;
}
.clock-option small {
  font-size: 11px;
  color: var(--text-tertiary);
}
.clock-check {
  position: absolute;
  top: 6px;
  right: 6px;
  display: grid;
  place-items: center;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--accent);
  color: var(--accent-contrast);
}
.clock-preview {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--text);
  font-variant-numeric: tabular-nums;
  line-height: 1.2;
}
.clock-plain {
  font-size: 22px;
  font-weight: 300;
}
.clock-bold {
  font-size: 28px;
  font-weight: 650;
  letter-spacing: -0.02em;
}
.clock-mono {
  font-family: var(--font-mono);
  font-size: 21px;
}
.clock-card {
  padding: 4px 10px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-subtle);
  font-family: var(--font-mono);
  font-size: 17px;
}
.theme-footnote {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 10px 4px 0;
  color: var(--text-tertiary);
  font-size: 12px;
}
</style>
