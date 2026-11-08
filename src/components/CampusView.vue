<script setup lang="ts">
import { computed, ref, type Component } from 'vue';
import PageHeader from './ui/PageHeader.vue';
import InlineAlert from './ui/InlineAlert.vue';
import CampusTabs, {
  type CampusPrimaryTab,
  type CampusSecondaryTab,
} from './campus/CampusTabs.vue';
import CampusNotices from './campus/CampusNotices.vue';
import CampusLms from './campus/CampusLms.vue';
import CampusTodos from './campus/CampusTodos.vue';
import CampusCourseFiles from './campus/CampusCourseFiles.vue';
import CampusGrades from './campus/CampusGrades.vue';
import CampusExams from './campus/CampusExams.vue';
import CampusEvaluations from './campus/CampusEvaluations.vue';
import CampusRooms from './campus/CampusRooms.vue';
import CampusCourses from './campus/CampusCourses.vue';
import CampusCard from './campus/CampusCard.vue';
import CampusLibrary from './campus/CampusLibrary.vue';
import CampusVenue from './campus/CampusVenue.vue';
import CampusAttendance from './campus/CampusAttendance.vue';
import CampusFitness from './campus/CampusFitness.vue';
import CampusProfile from './campus/CampusProfile.vue';
import CampusCalendar from './campus/CampusCalendar.vue';
import { ensureDirectSession } from '../campusSession';
import { campusTodoCount } from '../campusNav';

/**
 * 畅行校园 workspace (LAYOUT §3.3, version B): top-level partitions with a
 * second row for 学习 / 学业 / 生活. Sections the backend cannot serve stay in
 * the tabs, disabled with their reason, instead of disappearing.
 */
const props = withDefaults(
  defineProps<{
    name: string;
    accessMode: 'direct' | 'webvpn' | null;
    directSession?: boolean;
  }>(),
  { directSession: false },
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

const views: Record<string, Component> = {
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
  card: { title: '校园卡', group: 'life' },
  library: { title: '图书馆', group: 'life' },
  venues: { title: '场馆', group: 'life' },
  attendance: { title: '考勤', group: 'life' },
  fitness: { title: '体测', group: 'life' },
  profile: { title: '学籍档案', group: 'life' },
};

const GROUPS = [
  { id: 'study', title: '学习', children: ['lms', 'courseFiles'] },
  {
    id: 'academic',
    title: '学业',
    children: ['grades', 'exams', 'evaluations', 'rooms', 'courses', 'calendar'],
  },
  {
    id: 'life',
    title: '生活',
    children: ['card', 'library', 'venues', 'attendance', 'fitness', 'profile'],
  },
];

// Remember the last open section so the campus page never lands on an empty
// default. A disabled section is ignored, in case the list changed.
const LAST_SECTION = 'campus.lastSection';
const savedSection = (() => {
  try {
    const saved = localStorage.getItem(LAST_SECTION) ?? '';
    // Course materials were briefly called 资料库, and the user's own
    // 资料库 used to live here; both now map to 课程资料 or the 工作台.
    return saved === 'libraryFiles' || saved === 'localLibrary' ? 'courseFiles' : saved;
  } catch {
    return '';
  }
})();
if (savedSection && views[savedSection] && !vpnBlocked(savedSection)) tool.value = savedSection;

const activePrimary = computed(() => SECTIONS[tool.value]?.group ?? tool.value);
const activeGroup = computed(() => GROUPS.find((group) => group.id === activePrimary.value));
const secondaryItems = computed<CampusSecondaryTab[]>(
  () =>
    activeGroup.value?.children
      .filter((id) => views[id])
      .map((id) => ({ id, title: SECTIONS[id].title })) ?? [],
);
const primaryTabs = computed<CampusPrimaryTab[]>(() => [
  { id: 'todo', title: '待办', count: campusTodoCount.value },
  { id: 'notices', title: '通知' },
  { id: 'study', title: '学习', startGroup: true },
  { id: 'academic', title: '学业' },
  { id: 'life', title: '生活' },
]);
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
  const group = GROUPS.find((entry) => entry.id === id);
  if (!group) return;
  const remembered = lastByGroup.get(id);
  const target = remembered && group.children.includes(remembered) ? remembered : group.children[0];
  void select(target);
}
</script>

<template>
  <section class="desktop-page">
    <PageHeader :title="name" />

    <div class="campus-workspace">
      <CampusTabs
        :primary="primaryTabs"
        :active-primary="activePrimary"
        :secondary="secondaryItems"
        :active-secondary="tool"
        @primary="selectPrimary"
        @secondary="select"
      />
      <div class="campus-workspace-main">
        <InlineAlert v-if="error" tone="error" :title="error" />
        <CampusTodos v-if="tool === 'todo'" @open-section="select" />
        <component :is="views[tool]" v-else @capture="emit('capture', $event)" />
      </div>
    </div>
  </section>
</template>

<style scoped>
.campus-workspace-main {
  min-width: 0;
}
</style>
