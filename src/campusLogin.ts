import { ref } from 'vue';
import type { CampusDegree, CampusLogin, CampusLoginAction } from './types';

export interface LoginApi {
  startAccountLogin(degree: CampusDegree, requestId: string): Promise<CampusLogin>;
  advanceAccountLogin(id: string, action: CampusLoginAction): Promise<CampusLogin>;
  accountLoginState(id: string): Promise<CampusLogin>;
  accountLoginCaptcha(id: string): Promise<string>;
  accountLoginQr(id: string): Promise<string>;
  cancelAccountLogin(id: string): Promise<void>;
}

// Each new attempt invalidates earlier responses, including slow network probes.
export function createCampusLogin(api: LoginApi, success: () => void) {
  const flow = ref<CampusLogin | null>(null);
  const busy = ref(false);
  const error = ref('');
  const captchaImage = ref('');
  const qrImage = ref('');
  let qrTimer: ReturnType<typeof setInterval> | undefined;
  let pollingQr = false;
  let epoch = 0;
  let activeId: string | null = null;
  let disposed = false;
  const live = (version: number) => !disposed && epoch === version;
  const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
  async function release(id: string | null) {
    if (id) await api.cancelAccountLogin(id);
  }
  function stopQr() {
    if (qrTimer) clearInterval(qrTimer);
    qrTimer = undefined;
    qrImage.value = '';
    pollingQr = false;
  }
  async function loadQr() {
    const current = flow.value;
    if (disposed || !current) return;
    try {
      const image = await api.accountLoginQr(current.flowId);
      if (!disposed) qrImage.value = image;
    } catch (e) {
      if (!disposed) error.value = message(e);
    }
  }
  async function startQr() {
    stopQr();
    await loadQr();
    if (disposed) return;
    qrTimer = setInterval(() => {
      void pollQr();
    }, 1500);
  }
  async function pollQr() {
    const current = flow.value;
    if (pollingQr || disposed || !current || !['credentials', 'captcha'].includes(current.phase))
      return;
    pollingQr = true;
    const version = epoch;
    try {
      const next = await api.advanceAccountLogin(current.flowId, { type: 'pollQr' });
      if (!live(version)) return;
      await accept(next, version, false);
      if (!['credentials', 'captcha'].includes(next.phase)) stopQr();
    } catch (e) {
      if (!live(version)) return;
      const text = message(e);
      // Authorization that needs another factor cannot be completed by polling;
      // network hiccups should simply be retried by the next tick.
      if (/额外验证|阶段|重新开始/.test(text)) {
        error.value = text;
        stopQr();
      }
    } finally {
      pollingQr = false;
    }
  }
  async function accept(next: CampusLogin, version: number, refresh = true) {
    if (!live(version)) return;
    flow.value = next;
    if (next.phase !== 'credentials') stopQr();
    if (next.phase === 'success' && !next.savePending) {
      activeId = null;
      captchaImage.value = '';
      success();
    } else if (next.phase === 'captcha' && refresh) {
      const image = await api.accountLoginCaptcha(next.flowId);
      if (live(version)) captchaImage.value = image;
    } else if (next.phase !== 'captcha') captchaImage.value = '';
  }
  // The school's CAS can answer a password submission with an extra page that
  // needs no input; follow it here so a second "continue" click never appears.
  // Stop at a page that needs the user, and cap the chain to avoid a loop.
  async function followContinuations(version: number) {
    for (let step = 0; step < 2; step += 1) {
      const current = flow.value;
      if (!live(version) || !current || current.phase !== 'credentials' || !current.canContinue) {
        return;
      }
      try {
        const next = await api.advanceAccountLogin(current.flowId, { type: 'continue' });
        await accept(next, version);
      } catch (e) {
        if (live(version)) error.value = message(e);
        return;
      }
    }
    if (live(version) && flow.value?.phase === 'credentials' && flow.value.canContinue) {
      // Never leave a confirmation button behind: end this flow and let the
      // user start again, which is the only interaction the user expects.
      error.value = '学校返回了额外的确认页面，请重新开始登录。';
      await cancel();
    }
  }
  async function begin(degree: CampusDegree) {
    if (disposed) return;
    stopQr();
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
      await followContinuations(version);
    } catch (e) {
      if (live(version)) error.value = message(e);
    } finally {
      if (live(version)) busy.value = false;
    }
  }
  async function sendSmsCode(phone: string, captcha: string) {
    await submit({ type: 'sendSmsCode', phone, captcha });
  }
  async function submit(action: CampusLoginAction) {
    if (busy.value || !flow.value || disposed) return;
    const version = epoch;
    const id = flow.value.flowId;
    error.value = '';
    busy.value = true;
    try {
      await accept(await api.advanceAccountLogin(id, action), version);
      await followContinuations(version);
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
    stopQr();
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
  return {
    flow,
    busy,
    error,
    captchaImage,
    qrImage,
    begin,
    submit,
    sendSmsCode,
    refreshCaptcha,
    startQr,
    stopQr,
    cancel,
    dispose,
  };
}
