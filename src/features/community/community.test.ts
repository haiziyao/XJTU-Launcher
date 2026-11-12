import { describe, expect, it } from 'vitest';
import {
  AGENT_REASON_TEXT,
  parseFavoriteIds,
  resourceTabs,
  searchSections,
  sitesForTab,
  toggleFavoriteId,
} from './community';
import type { ResourceCatalog } from '../../types';

describe('community section search', () => {
  it('returns the matching section for an exact name', () => {
    expect(searchSections('课程攻略').map((section) => section.id)).toEqual(['courses']);
    expect(searchSections('资源站点').map((section) => section.id)).toEqual(['links']);
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

describe('community favorites', () => {
  it('drops malformed, unknown and repeated ids from storage', () => {
    expect(parseFavoriteIds(null)).toEqual([]);
    expect(parseFavoriteIds('not json')).toEqual([]);
    expect(parseFavoriteIds('{"clubs":true}')).toEqual([]);
    expect(parseFavoriteIds('["clubs","clubs","nope",3]')).toEqual(['clubs']);
    expect(parseFavoriteIds('["links","clubs"]')).toEqual(['links', 'clubs']);
  });

  it('adds a favorite at the end and removes it without touching the rest', () => {
    expect(toggleFavoriteId(['clubs'], 'links')).toEqual(['clubs', 'links']);
    expect(toggleFavoriteId(['clubs', 'links'], 'clubs')).toEqual(['links']);
  });
});

describe('resource site tabs', () => {
  const catalog: ResourceCatalog = {
    version: 1,
    updatedAt: '2026-11-11',
    categories: [
      { id: 'study', title: '学习' },
      { id: 'life', title: '生活' },
    ],
    sites: [
      {
        id: 'a',
        title: 'A',
        url: 'https://a.xjtu.edu.cn/',
        description: '',
        categories: ['study'],
        tags: [],
        featured: true,
        icon: null,
      },
      {
        id: 'b',
        title: 'B',
        url: 'https://b.xjtu.edu.cn/',
        description: '',
        categories: ['life'],
        tags: [],
        featured: false,
        icon: null,
      },
      {
        id: 'c',
        title: 'C',
        url: 'https://c.xjtu.edu.cn/',
        description: '',
        categories: ['study', 'life'],
        tags: [],
        featured: true,
        icon: null,
      },
    ],
  };

  it('puts 推荐 before the catalog categories', () => {
    expect(resourceTabs(catalog)).toEqual([
      { id: 'recommended', title: '推荐' },
      { id: 'study', title: '学习' },
      { id: 'life', title: '生活' },
    ]);
    expect(resourceTabs(null)).toEqual([{ id: 'recommended', title: '推荐' }]);
  });

  it('shows featured sites on 推荐 and category members elsewhere', () => {
    expect(sitesForTab(catalog, 'recommended').map((site) => site.id)).toEqual(['a', 'c']);
    expect(sitesForTab(catalog, 'study').map((site) => site.id)).toEqual(['a', 'c']);
    expect(sitesForTab(catalog, 'life').map((site) => site.id)).toEqual(['b', 'c']);
    expect(sitesForTab(catalog, 'missing')).toEqual([]);
    expect(sitesForTab(null, 'recommended')).toEqual([]);
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
