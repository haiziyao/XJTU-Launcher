<script setup lang="ts">
/** One top-level campus partition; `startGroup` draws the divider before 学习. */
export interface CampusPrimaryTab {
  id: string;
  title: string;
  count?: number | null;
  startGroup?: boolean;
}
export interface CampusSecondaryTab {
  id: string;
  title: string;
}

/**
 * Campus navigation (LAYOUT §3.3, version B): one row of primary partitions and,
 * for 学习 / 学业 / 生活, a second row of their sections. Replaces the left rail
 * so the content gets the full window width.
 */
withDefaults(
  defineProps<{
    primary: CampusPrimaryTab[];
    activePrimary: string;
    secondary?: CampusSecondaryTab[];
    activeSecondary?: string;
  }>(),
  { secondary: () => [], activeSecondary: '' },
);
const emit = defineEmits<{ primary: [id: string]; secondary: [id: string] }>();
</script>

<template>
  <nav class="campus-tabs" aria-label="畅行校园分区">
    <div class="campus-tabs-primary" role="tablist" aria-label="分区">
      <template v-for="tab in primary" :key="tab.id">
        <span v-if="tab.startGroup" class="tabs-divider" aria-hidden="true"></span>
        <button
          type="button"
          role="tab"
          :aria-selected="activePrimary === tab.id"
          :class="{ active: activePrimary === tab.id }"
          @click="emit('primary', tab.id)"
        >
          {{ tab.title }}
          <span v-if="tab.count != null" class="tabs-count">{{ tab.count }}</span>
        </button>
      </template>
    </div>
    <div v-if="secondary.length" class="campus-tabs-secondary" role="tablist" aria-label="二级分区">
      <button
        v-for="tab in secondary"
        :key="tab.id"
        type="button"
        role="tab"
        :aria-selected="activeSecondary === tab.id"
        :class="{ active: activeSecondary === tab.id }"
        @click="emit('secondary', tab.id)"
      >
        {{ tab.title }}
      </button>
    </div>
  </nav>
</template>

<style scoped>
.campus-tabs {
  margin-bottom: 18px;
}
.campus-tabs-primary,
.campus-tabs-secondary {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-wrap: wrap;
}
.campus-tabs-primary button,
.campus-tabs-secondary button {
  border: 0;
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  cursor: pointer;
  white-space: nowrap;
  transition:
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}
.campus-tabs-primary button {
  display: inline-flex;
  align-items: center;
  height: 32px;
  padding: 0 12px;
  border-radius: var(--radius);
  font-size: 13.5px;
  font-weight: 500;
}
.campus-tabs-primary button:hover {
  background: var(--accent-wash);
  color: var(--text);
}
.campus-tabs-primary button.active {
  background: var(--accent-soft);
  color: var(--accent-text);
}
.tabs-count {
  margin-left: 6px;
  color: var(--text-tertiary);
  font-size: 11px;
  font-weight: 500;
  font-variant-numeric: tabular-nums;
}
.campus-tabs-primary button.active .tabs-count {
  color: var(--accent-text);
  opacity: 0.75;
}
.tabs-divider {
  width: 1px;
  height: 16px;
  margin: 0 8px;
  background: var(--divider);
}
.campus-tabs-secondary {
  gap: 4px;
  margin-top: 6px;
  padding-top: 8px;
  border-top: 1px solid var(--divider);
}
.campus-tabs-secondary button {
  height: 28px;
  padding: 0 10px;
  border-radius: var(--radius-sm);
  font-size: 12.5px;
}
.campus-tabs-secondary button:hover {
  background: var(--accent-wash);
  color: var(--text);
}
.campus-tabs-secondary button.active {
  color: var(--accent-text);
  font-weight: 600;
}
</style>
