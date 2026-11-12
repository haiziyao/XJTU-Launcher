<script setup lang="ts">
import { computed, ref, type Component } from 'vue';
import InlineAlert from '../../components/ui/InlineAlert.vue';
import CampusOverview from './components/CampusOverview.vue';
import CampusTodos from './components/CampusTodos.vue';
import CampusNotices from './components/CampusNotices.vue';
import CampusLms from './components/CampusLms.vue';
import CampusCourseFiles from './components/CampusCourseFiles.vue';
import CampusGrades from './components/CampusGrades.vue';
import CampusExams from './components/CampusExams.vue';
import CampusEvaluations from './components/CampusEvaluations.vue';
import CampusRooms from './components/CampusRooms.vue';
import CampusCourses from './components/CampusCourses.vue';
import CampusCard from './components/CampusCard.vue';
import CampusLibrary from './components/CampusLibrary.vue';
import CampusVenue from './components/CampusVenue.vue';
import CampusAttendance from './components/CampusAttendance.vue';
import CampusFitness from './components/CampusFitness.vue';
import CampusProfile from './components/CampusProfile.vue';
import CampusCalendar from './components/CampusCalendar.vue';
import { ensureDirectSession } from './campusSession';
import { campusNoticeCount, campusTodoCount } from './campusNav';
import { provideCampusHeader } from './campusHeader';

/**
 * 畅行校园 workspace (layout v2, direction B): one header row with the primary
 * partitions, one row with the section tabs, then a three-column master–detail
 * body. Sections the backend cannot serve stay in the tabs, disabled with their
 * reason, instead of disappearing.
 */
const props = withDefaults(
  defineProps<{
    name: string;
    /** `all` is 畅行校园; the other two are the focused campus plugins. */
    preset?: 'all' | 'reservations' | 'profile';
    accessMode: 'direct' | 'webvpn' | null;
    directSession?: boolean;
    /** The avatar the user configured, for pages that show the student. */
    avatar?: string | null;
  }>(),
  { preset: 'all', directSession: false, avatar: null },
);
const emit = defineEmits<{
  'direct-session': [];
  capture: [target: { activity: number; course: number | null; title: string }];
}>();

const tool = ref('notices');
const error = ref('');
const directReady = ref(false);
const hasDirect = computed(() => props.directSession || directReady.value);
const onWebVpn = computed(() => props.accessMode === 'webvpn');
// These services reject the WebVPN session itself; a direct login unlocks them.
// The venue and library systems use their own logins and are always attempted.
const DIRECT_ONLY = ['card', 'fitness'];
const vpnBlocked = (id: string) => onWebVpn.value && !hasDirect.value && DIRECT_ONLY.includes(id);

// Tools and meta for the section row are rendered by the page but belong to
// the shell's row; pages teleport into these two elements.
const metaTarget = ref<HTMLElement | null>(null);
const toolsTarget = ref<HTMLElement | null>(null);
provideCampusHeader({ meta: metaTarget, tools: toolsTarget });

const views: Record<string, Component> = {
  overview: CampusOverview,
  notices: CampusNotices,
  todo: CampusTodos,
  lms: CampusLms,
  courseFiles: CampusCourseFiles,
  grades: CampusGrades,
  exams: CampusExams,
  evaluations: CampusEvaluations,
  rooms: CampusRooms,
  courses: CampusCourses,
  calendar: CampusCalendar,
  card: CampusCard,
  library: CampusLibrary,
  venues: CampusVenue,
  attendance: CampusAttendance,
  fitness: CampusFitness,
  profile: CampusProfile,
};

const SECTIONS: Record<string, { title: string; group?: string }> = {
  overview: { title: '概览', group: 'me' },
  todo: { title: '待办' },
  notices: { title: '通知' },
  lms: { title: '思源学堂', group: 'study' },
  courseFiles: { title: '课程资料', group: 'study' },
  grades: { title: '成绩', group: 'academic' },
  exams: { title: '考试', group: 'academic' },
  evaluations: { title: '评教', group: 'academic' },
  rooms: { title: '空闲教室', group: 'academic' },
  courses: { title: '全校课程', group: 'academic' },
  calendar: { title: '校历', group: 'academic' },
  card: { title: '校园卡', group: 'me' },
  library: { title: '图书馆', group: 'booking' },
  venues: { title: '场馆', group: 'booking' },
  attendance: { title: '考勤', group: 'academic' },
  fitness: { title: '体测', group: 'me' },
  profile: { title: '学籍档案', group: 'me' },
};

// 全校课程 is hidden from the tabs for now (2026-11-12). The page, view
// mapping and section entry stay in the code, so restoring the tab is only a
// matter of putting 'courses' back into the list.
const HIDDEN_SECTIONS = new Set(['courses']);

type Group = { id: string; title: string; children: string[] };

// 生活 is gone: 预约中心 and 个人画像 are separate default-enabled plugins
// (see plugins/cn.xjtu.campus-*). 考勤 stays with 学业.
const PRESET_GROUPS: Record<'all' | 'reservations' | 'profile', Group[]> = {
  all: [
    { id: 'study', title: '学习', children: ['lms', 'courseFiles'] },
    {
      id: 'academic',
      title: '学业',
      children: ['grades', 'exams', 'evaluations', 'rooms', 'calendar', 'attendance'],
    },
  ],
  reservations: [{ id: 'booking', title: '预约', children: ['library', 'venues'] }],
  profile: [{ id: 'me', title: '个人画像', children: ['overview', 'card', 'fitness', 'profile'] }],
};
const groups = computed(() => PRESET_GROUPS[props.preset]);

// Remember the last open section so the campus page never lands on an empty
// default. A disabled section is ignored, in case the list changed. Each
// preset keeps its own preference.
const LAST_SECTION = `campus.lastSection.${props.preset}`;
const savedSection = (() => {
  try {
    // Older builds kept one shared key; 畅行校园 still honours it once so an
    // upgrade does not reset the last section to 通知.
    const stored =
      localStorage.getItem(LAST_SECTION) ??
      (props.preset === 'all' ? localStorage.getItem('campus.lastSection') : null) ??
      '';
    // Course materials were briefly called 资料库, and the user's own
    // 资料库 used to live here; both now map to 课程资料 or the 工作台.
    return stored === 'libraryFiles' || stored === 'localLibrary' ? 'courseFiles' : stored;
  } catch {
    return '';
  }
})();
if (!groups.value.some((group) => group.children.includes(tool.value))) {
  // Default to a section that can load without a direct login; 个人画像 would
  // otherwise open on 校园卡, which a WebVPN-only session cannot read.
  const children = groups.value[0].children;
  tool.value = children.find((id) => !vpnBlocked(id)) ?? children[0];
}
if (
  savedSection &&
  groups.value.some((group) => group.children.includes(savedSection)) &&
  !HIDDEN_SECTIONS.has(savedSection) &&
  !vpnBlocked(savedSection)
) {
  tool.value = savedSection;
}

const activePrimary = computed(() => SECTIONS[tool.value]?.group ?? tool.value);
const activeGroup = computed(() => groups.value.find((group) => group.id === activePrimary.value));
const secondaryItems = computed(
  () =>
    activeGroup.value?.children
      .filter((id) => views[id])
      .map((id) => ({ id, title: SECTIONS[id].title })) ?? [],
);
// The focused plugins open one group, so their first row is just the plugin
// name and the section tabs below; only 畅行校园 needs partition tabs.
const primaryTabs = computed<{ id: string; title: string; count?: number | null }[]>(() =>
  props.preset === 'all'
    ? [
        { id: 'todo', title: '待办', count: campusTodoCount.value },
        { id: 'notices', title: '通知', count: campusNoticeCount.value },
        { id: 'study', title: '学习' },
        { id: 'academic', title: '学业' },
      ]
    : [],
);

// Quick switches inside a group (学习 → 资料库 → 学习) return to the section the
// user last had open there, not always the first child.
const lastByGroup = new Map<string, string>();
if (activeGroup.value && savedSection) lastByGroup.set(activeGroup.value.id, tool.value);

/** Direct-only sections log in on demand; a failed login explains itself. */
async function openDirect(id: string): Promise<boolean> {
  if (!vpnBlocked(id)) return true;
  try {
    await ensureDirectSession();
    directReady.value = true;
    emit('direct-session');
    return true;
  } catch (e) {
    console.error('[campus] direct login', id, e);
    error.value = e instanceof Error ? e.message : String(e);
    return false;
  }
}

async function select(id: string) {
  if (!views[id]) return;
  error.value = '';
  if (!(await openDirect(id))) return;
  tool.value = id;
  error.value = '';
  const group = SECTIONS[id]?.group;
  if (group) lastByGroup.set(group, id);
  try {
    localStorage.setItem(LAST_SECTION, id);
  } catch {
    // A missing preference only costs the default section on the next launch.
  }
}

function selectPrimary(id: string) {
  if (SECTIONS[id] && !SECTIONS[id].group) {
    void select(id);
    return;
  }
  const group = groups.value.find((entry) => entry.id === id);
  if (!group) return;
  const remembered = lastByGroup.get(id);
  const target = remembered && group.children.includes(remembered) ? remembered : group.children[0];
  void select(target);
}
</script>

<template>
  <section class="campus-v2">
    <header class="cv-top">
      <h1 class="cv-title">{{ name }}</h1>
      <nav v-if="primaryTabs.length" class="cv-ptabs" aria-label="校园分区">
        <template v-for="(tab, index) in primaryTabs" :key="tab.id">
          <span v-if="index === 2" class="cv-vsep" aria-hidden="true"></span>
          <button
            type="button"
            class="cv-ptab"
            :class="{ on: activePrimary === tab.id }"
            :aria-current="activePrimary === tab.id ? 'page' : undefined"
            @click="selectPrimary(tab.id)"
          >
            {{ tab.title }}<span v-if="tab.count != null" class="n">{{ tab.count }}</span>
          </button>
        </template>
      </nav>
      <div class="cv-top-right"></div>
    </header>

    <!-- Grouped sections keep this row for their tabs; the page meta then sits
         with the tools on the right instead of being wedged among the tabs. -->
    <div class="cv-sub">
      <nav class="cv-tabs" aria-label="校园功能">
        <button
          v-for="item in secondaryItems"
          :key="item.id"
          type="button"
          class="cv-stab"
          :class="{ on: tool === item.id }"
          :aria-current="tool === item.id ? 'page' : undefined"
          @click="select(item.id)"
        >
          {{ item.title }}
        </button>
      </nav>
      <div ref="metaTarget" class="cv-head-meta" :class="{ push: !!activeGroup }"></div>
      <div ref="toolsTarget" class="cv-tools"></div>
    </div>

    <InlineAlert v-if="error" tone="error" :title="error" />

    <div class="cv-body">
      <CampusOverview
        v-if="tool === 'overview'"
        :access-mode="accessMode"
        :direct-session="hasDirect"
        :avatar="avatar"
        @open-section="select"
      />
      <CampusTodos v-else-if="tool === 'todo'" @open-section="select" />
      <CampusProfile v-else-if="tool === 'profile'" :avatar="avatar" />
      <component :is="views[tool]" v-else @capture="emit('capture', $event)" />
    </div>
  </section>
</template>
