import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { SchoolQuery } from './types';
import { directQuery, ensureDirectSession } from './campusSession';

const api = vi.hoisted(() => ({
  refreshDirectSession: vi.fn(),
  schoolQuery: vi.fn(),
}));
vi.mock('./bridge', () => ({ bridge: api }));

beforeEach(() => {
  api.refreshDirectSession.mockReset();
  api.schoolQuery.mockReset();
});

describe('ensureDirectSession', () => {
  it('shares one non-forced login between concurrent callers', async () => {
    let finish!: (value: boolean) => void;
    api.refreshDirectSession.mockReturnValue(
      new Promise<boolean>((resolve) => {
        finish = resolve;
      }),
    );
    const first = ensureDirectSession();
    const second = ensureDirectSession();
    expect(api.refreshDirectSession).toHaveBeenCalledTimes(1);
    expect(api.refreshDirectSession).toHaveBeenCalledWith(false);
    finish(true);
    await Promise.all([first, second]);
  });

  it('keeps a forced renewal separate from a pending warm-up', async () => {
    let finishWarm!: (value: boolean) => void;
    api.refreshDirectSession
      .mockReturnValueOnce(
        new Promise<boolean>((resolve) => {
          finishWarm = resolve;
        }),
      )
      .mockResolvedValueOnce(true);
    const warm = ensureDirectSession();
    const forced = ensureDirectSession(true);
    expect(api.refreshDirectSession.mock.calls).toEqual([[false], [true]]);
    finishWarm(true);
    await Promise.all([warm, forced]);
  });

  it('renews once with force and retries when a direct query expires', async () => {
    const card: SchoolQuery = { type: 'card' };
    api.schoolQuery
      .mockRejectedValueOnce(new Error('直连会话已失效，请重新登录'))
      .mockResolvedValueOnce({ balance: 1 });
    api.refreshDirectSession.mockResolvedValue(true);
    await expect(directQuery(card)).resolves.toEqual({ balance: 1 });
    expect(api.refreshDirectSession).toHaveBeenCalledWith(true);
    expect(api.schoolQuery).toHaveBeenCalledTimes(2);
  });

  it('passes unrelated errors through without a renewal', async () => {
    const card: SchoolQuery = { type: 'card' };
    api.schoolQuery.mockRejectedValueOnce(new Error('无法连接校园卡服务'));
    await expect(directQuery(card)).rejects.toThrow('无法连接校园卡服务');
    expect(api.refreshDirectSession).not.toHaveBeenCalled();
  });
});
