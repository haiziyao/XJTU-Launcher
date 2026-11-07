<script setup lang="ts">
import { onBeforeUnmount, watch } from 'vue';
import { IconX } from '@tabler/icons-vue';
import UiIconButton from './UiIconButton.vue';

/**
 * Card that opens from a settings row: one detail view at a time, closed by the
 * button, a click outside or Esc. Used by the Agent lists so the page itself
 * stays a plain list instead of stacking every form at once.
 */
const props = defineProps<{ open: boolean; title: string; wide?: boolean }>();
const emit = defineEmits<{ close: [] }>();

function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape') emit('close');
}
watch(
  () => props.open,
  (open) => {
    if (open) window.addEventListener('keydown', onKey);
    else window.removeEventListener('keydown', onKey);
  },
  { immediate: true },
);
onBeforeUnmount(() => window.removeEventListener('keydown', onKey));
</script>
<template>
  <Teleport to="body">
    <div v-if="open" class="dialog-backdrop" @click.self="emit('close')">
      <section
        class="dialog-card"
        :class="{ wide }"
        role="dialog"
        aria-modal="true"
        :aria-label="title"
      >
        <header class="dialog-head">
          <h2>{{ title }}</h2>
          <UiIconButton label="关闭" @click="emit('close')"><IconX :size="16" /></UiIconButton>
        </header>
        <div class="dialog-body"><slot /></div>
        <footer v-if="$slots.footer" class="dialog-foot"><slot name="footer" /></footer>
      </section>
    </div>
  </Teleport>
</template>
<style scoped>
.dialog-backdrop {
  position: fixed;
  z-index: 130;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(15, 23, 42, 0.32);
}
.dialog-card {
  display: flex;
  flex-direction: column;
  gap: 14px;
  width: 460px;
  max-width: 100%;
  max-height: calc(100vh - 64px);
  padding: 20px 22px 18px;
  border-radius: var(--radius-lg);
  background: var(--surface);
  box-shadow: var(--shadow-lg);
}
.dialog-card.wide {
  width: 620px;
}
.dialog-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}
.dialog-head h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 650;
}
.dialog-body {
  display: grid;
  gap: 12px;
  min-height: 0;
  overflow-y: auto;
  color: var(--text-secondary);
  font-size: 13px;
  line-height: 1.65;
}
.dialog-foot {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
