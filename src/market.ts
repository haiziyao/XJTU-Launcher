import { ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
export const marketBusy = ref('');
export const marketProgress = ref<number | null>(null);
export const marketError = ref('');
let initialized = false;
export async function initializeMarket(reload: () => Promise<void>) {
  if (initialized) return;
  initialized = true;
  await listen<{ id: string; received: number; total: number | null }>('plugin-download', (e) => {
    if (e.payload.id === marketBusy.value)
      marketProgress.value = e.payload.total
        ? Math.min(100, Math.round((e.payload.received / e.payload.total) * 100))
        : null;
  });
  await listen('plugins-changed', () => {
    void reload();
  });
}
