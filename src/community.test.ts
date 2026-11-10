import { describe, expect, it } from 'vitest';
import { AGENT_REASON_TEXT, searchSections } from './community';

describe('community section search', () => {
  it('returns the matching section for an exact name', () => {
    expect(searchSections('课程攻略').map((section) => section.id)).toEqual(['courses']);
    expect(searchSections('资源导航').map((section) => section.id)).toEqual(['links']);
  });

  it('matches keywords inside a section', () => {
    expect(searchSections('食堂').map((section) => section.id)).toEqual(['life']);
    expect(searchSections('课程号').map((section) => section.id)).toEqual(['courses']);
    expect(searchSections('保研').map((section) => section.id)).toEqual(['career']);
  });

  it('matches multiple tokens with AND semantics and ignores whitespace', () => {
    expect(searchSections('课程 选课').map((section) => section.id)).toEqual(['courses']);
    expect(searchSections('课程 食堂')).toEqual([]);
  });

  it('returns no sections for an unknown query', () => {
    expect(searchSections('不存在的内容')).toEqual([]);
  });

  it('returns no sections for an empty or whitespace query', () => {
    expect(searchSections('')).toEqual([]);
    expect(searchSections('   ')).toEqual([]);
  });
});

describe('agent availability copy', () => {
  it('uses the exact unreachable message', () => {
    expect(AGENT_REASON_TEXT.unreachable).toBe('Agent 服务不可用');
  });

  it('leaves ready without a message', () => {
    expect(AGENT_REASON_TEXT.ready).toBe('');
  });
});
