import { ref } from 'vue';
import type { CampusDegree, CampusLogin, CampusLoginAction } from './types';

export interface LoginApi {
  startAccountLogin(degree: CampusDegree, requestId: string): Promise<CampusLogin>;
  advanceAccountLogin(id: string, action: CampusLoginAction): Promise<CampusLogin>;
  accountLoginState(id: string): Promise<CampusLogin>;
  accountLoginCaptcha(id: string): Promise<string>;
  cancelAccountLogin(id: string): Promise<void>;
}

// Each new attempt invalidates earlier responses, including slow network probes.
export function createCampusLogin(api: LoginApi, success: () => void) {
  const flow = ref<CampusLogin | null>(null);
  const busy = ref(false);
  const error = ref('');
  const captchaImage = ref('');
  let epoch = 0;
  let activeId: string | null = null;
  let disposed = false;
  const live = (version: number) => !disposed && epoch === version;
  const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
  async function release(id: string | null) {
    if (id) await api.cancelAccountLogin(id);
  }
  async function accept(next: CampusLogin, version: number, refresh = true) {
    if (!live(version)) return;
    flow.value = next;
    if (next.phase === 'success' && !next.savePending) {
      activeId = null;
      captchaImage.value = '';
      success();
    } else if (next.phase === 'captcha' && refresh) {
      const image = await api.accountLoginCaptcha(next.flowId);
      if (live(version)) captchaImage.value = image;
    } else if (next.phase !== 'captcha') captchaImage.value = '';
  }
  async function begin(degree: CampusDegree) {
    if (disposed) return;
    const version = ++epoch;
    const previous = activeId;
    activeId = null;
    flow.value = null;
    error.value = '';
    captchaImage.value = '';
    busy.value = true;
    try {
      await release(previous);
      if (!live(version)) return;
      const id = crypto.randomUUID();
      activeId = id;
      const next = await api.startAccountLogin(degree, id);
      if (!live(version)) {
        await release(id);
        return;
      }
      await accept(next, version);
    } catch (e) {
      if (live(version)) error.value = message(e);
    } finally {
      if (live(version)) busy.value = false;
    }
  }
  async function submit(action: CampusLoginAction) {
    if (busy.value || !flow.value || disposed) return;
    const version = epoch;
    const id = flow.value.flowId;
    error.value = '';
    busy.value = true;
    try {
      await accept(await api.advanceAccountLogin(id, action), version);
    } catch (e) {
      if (!live(version)) return;
      error.value = message(e);
      // The backend may have progressed before a network or persistence error.
      try {
        await accept(await api.accountLoginState(id), version, false);
      } catch {
        /* Preserve the actionable original error and the restart control. */
      }
    } finally {
      if (live(version)) busy.value = false;
    }
  }
  async function refreshCaptcha() {
    if (busy.value || !flow.value || disposed) return;
    const version = epoch;
    busy.value = true;
    error.value = '';
    try {
      const image = await api.accountLoginCaptcha(flow.value.flowId);
      if (live(version)) captchaImage.value = image;
    } catch (e) {
      if (live(version)) error.value = message(e);
    } finally {
      if (live(version)) busy.value = false;
    }
  }
  async function cancel() {
    const version = ++epoch;
    const id = activeId;
    activeId = null;
    flow.value = null;
    captchaImage.value = '';
    busy.value = false;
    try {
      await release(id);
    } catch (e) {
      if (live(version)) error.value = message(e);
    }
  }
  function dispose() {
    disposed = true;
    void cancel();
  }
  return { flow, busy, error, captchaImage, begin, submit, refreshCaptcha, cancel, dispose };
}
