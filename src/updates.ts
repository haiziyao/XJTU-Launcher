import { ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { bridge } from './bridge';
import type { UpdateStatus } from './types';
export const updateStatus = ref<UpdateStatus | null>(null);
let initialized: Promise<void> | undefined;
let stop: (() => void) | undefined;
export function initializeUpdates(autoCheck = false): Promise<void> {
  if (!initialized)
    initialized = (async () => {
      stop = await listen<UpdateStatus>('app-update', (e) => (updateStatus.value = e.payload));
      updateStatus.value = await bridge.updateStatus();
      if (autoCheck && updateStatus.value.phase === 'idle')
        updateStatus.value = await bridge.checkUpdate();
    })().catch((error) => {
      stop?.();
      stop = undefined;
      initialized = undefined;
      throw error;
    });
  return initialized;
}
