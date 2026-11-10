import type { Component } from 'vue';
import { BookOpen, Compass, Link2, MapPin, Users } from 'lucide-vue-next';
import type { AgentSearchReason } from '../../types';

export type CommunitySectionId = 'clubs' | 'courses' | 'life' | 'career' | 'links';

export interface CommunitySection {
  id: CommunitySectionId;
  title: string;
  description: string;
  keywords: string[];
  icon: Component;
  /** Decorative accent used by the card media block. */
  accent: string;
  /** Soft tint layered under the accent. */
  accentSoft: string;
}

export const COMMUNITY_SECTIONS: CommunitySection[] = [
  {
    id: 'clubs',
    title: '社团',
    description: '社团介绍、活动与招新信息',
    keywords: ['社团', '招新', '活动', '兴趣', '学生组织', '公众号'],
    icon: Users,
    accent: '#3b6fd8',
    accentSoft: '#dfe9fb',
  },
  {
    id: 'courses',
    title: '课程攻略',
    description: '课程号、学习建议与选课经验',
    keywords: ['课程', '课程号', '选课', '学习建议', '先修', '考核'],
    icon: BookOpen,
    accent: '#6a5ac9',
    accentSoft: '#e8e3fb',
  },
  {
    id: 'life',
    title: '校园生活',
    description: '吃住行、校园设施与办事指南',
    keywords: ['校园生活', '食堂', '宿舍', '校医院', '快递', '场馆', '办事', '吃住行'],
    icon: MapPin,
    accent: '#2c8653',
    accentSoft: '#ddf1e6',
  },
  {
    id: 'career',
    title: '生涯分享',
    description: '升学、就业、竞赛与成长经验',
    keywords: ['生涯', '升学', '就业', '保研', '考研', '出国', '实习', '竞赛', '科研'],
    icon: Compass,
    accent: '#a8641c',
    accentSoft: '#f8e8d5',
  },
  {
    id: 'links',
    title: '资源导航',
    description: '官方与第三方资源站引用',
    keywords: ['资源', '导航', '官网', '图书馆', '教务处', '正版软件', '开源工具', '外部链接'],
    icon: Link2,
    accent: '#566176',
    accentSoft: '#e4e8ef',
  },
];

/**
 * Local navigation search: every whitespace-separated token must occur in the
 * section title, description or keywords. Matching ignores case, and the
 * compacted text also lets "课程 号" find "课程号".
 */
export function searchSections(query: string): CommunitySection[] {
  const trimmed = query.trim();
  if (!trimmed) return [];
  const tokens = trimmed.toLowerCase().split(/\s+/).filter(Boolean);
  return COMMUNITY_SECTIONS.filter((section) => {
    const text = [section.title, section.description, ...section.keywords].join(' ').toLowerCase();
    const compact = text.replace(/\s+/g, '');
    return tokens.every((token) => text.includes(token) || compact.includes(token));
  });
}

export const AGENT_REASON_TEXT: Record<AgentSearchReason, string> = {
  ready: '',
  'plugin-disabled': 'Agent 未启用',
  disabled: '未开启搜索服务 Agent',
  'no-model': '未配置模型服务',
  unreachable: 'Agent 服务不可用',
};
