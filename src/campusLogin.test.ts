import { describe, expect, it, vi } from 'vitest';
import { createCampusLogin, type LoginApi } from './campusLogin';
import type { CampusLogin } from './types';

function view(id: string, fields: Partial<CampusLogin> = {}): CampusLogin {
  return {
    flowId: id,
    accessMode: 'direct',
    step: 'campus',
    phase: 'credentials',
    phone: null,
    choices: [],
    message: null,
    canContinue: false,
    retryAfterSeconds: 0,
    savePending: false,
    ...fields,
  };
}
function api() {
  return {
    startAccountLogin: vi.fn(async (_degree, id) => view(id)),
    advanceAccountLogin: vi.fn(async (id) => view(id)),
    accountLoginState: vi.fn(async (id) => view(id)),
    accountLoginCaptcha: vi.fn(async () => 'data:image/png;base64,TEST'),
    cancelAccountLogin: vi.fn(async () => {}),
  } satisfies LoginApi;
}
describe('campus login lifecycle', () => {
  it('cancels a pending probe and discards its late response', async () => {
    const service = api();
    let resolve!: (value: CampusLogin) => void;
    service.startAccountLogin.mockImplementationOnce(() => new Promise((r) => (resolve = r)));
    const success = vi.fn();
    const login = createCampusLogin(service, success);
    const start = login.begin('undergraduate');
    await vi.waitFor(() => expect(service.startAccountLogin).toHaveBeenCalledOnce());
    const id = service.startAccountLogin.mock.calls[0][1];
    await login.cancel();
    resolve(view(id, { phase: 'success' }));
    await start;
    expect(login.flow.value).toBeNull();
    expect(success).not.toHaveBeenCalled();
    expect(service.cancelAccountLogin).toHaveBeenCalledWith(id);
    expect(login.busy.value).toBe(false);
  });
  it('follows an inputless confirmation page without a second click', async () => {
    const service = api();
    const success = vi.fn();
    const login = createCampusLogin(service, success);
    await login.begin('undergraduate');
    service.advanceAccountLogin
      .mockImplementationOnce(async (flowId) => view(flowId, { canContinue: true }))
      .mockImplementationOnce(async (flowId) => view(flowId, { phase: 'success' }));
    await login.submit({
      type: 'credentials',
      username: 'test',
      password: 'test-password',
      rememberPassword: false,
    });
    expect(service.advanceAccountLogin).toHaveBeenCalledTimes(2);
    expect(service.advanceAccountLogin.mock.calls[1][1]).toEqual({ type: 'continue' });
    expect(success).toHaveBeenCalledTimes(1);
    expect(login.busy.value).toBe(false);
    login.dispose();
  });
  it('restores the backend phase after partial network failure without sending SMS', async () => {
    const service = api();
    const login = createCampusLogin(service, vi.fn());
    await login.begin('undergraduate');
    service.advanceAccountLogin.mockRejectedValueOnce(new Error('network failed'));
    service.accountLoginState.mockImplementationOnce(async (id) =>
      view(id, { phase: 'mfa', phone: '138****1234' }),
    );
    await login.submit({
      type: 'credentials',
      username: 'test',
      password: 'test-password',
      rememberPassword: false,
    });
    expect(login.flow.value?.phase).toBe('mfa');
    expect(login.error.value).toBe('network failed');
    expect(service.advanceAccountLogin).toHaveBeenCalledTimes(1);
    expect(service.accountLoginCaptcha).not.toHaveBeenCalled();
    login.dispose();
  });
  it('does not report success before a failed local save is retried', async () => {
    const service = api();
    const success = vi.fn();
    const login = createCampusLogin(service, success);
    await login.begin('undergraduate');
    service.advanceAccountLogin.mockRejectedValueOnce(new Error('keychain unavailable'));
    service.accountLoginState.mockImplementationOnce(async (id) =>
      view(id, { phase: 'success', savePending: true }),
    );
    await login.submit({ type: 'continue' });
    expect(success).not.toHaveBeenCalled();
    expect(login.flow.value?.savePending).toBe(true);
    service.advanceAccountLogin.mockImplementationOnce(async (id) =>
      view(id, { phase: 'success' }),
    );
    await login.submit({ type: 'continue' });
    expect(success).toHaveBeenCalledOnce();
    login.dispose();
  });
  it('loads captcha only when needed and prevents overlapping submissions', async () => {
    const service = api();
    const login = createCampusLogin(service, vi.fn());
    await login.begin('undergraduate');
    expect(service.accountLoginCaptcha).not.toHaveBeenCalled();
    service.advanceAccountLogin.mockImplementationOnce(async (id) =>
      view(id, { phase: 'captcha' }),
    );
    const first = login.submit({ type: 'continue' });
    await login.submit({ type: 'continue' });
    await first;
    expect(service.advanceAccountLogin).toHaveBeenCalledTimes(1);
    expect(service.accountLoginCaptcha).toHaveBeenCalledTimes(1);
    expect(login.captchaImage.value).toContain('data:image/');
    login.dispose();
  });
});
