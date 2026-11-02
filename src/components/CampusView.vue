<script setup lang="ts">
import { computed, ref, type Component } from 'vue';
import {
  Activity,
  BarChart3,
  Bell,
  BookMarked,
  BookOpen,
  BookOpenText,
  CalendarCheck,
  CalendarDays,
  ClipboardCheck,
  ClipboardList,
  CreditCard,
  DoorOpen,
  FolderArchive,
  FolderOpen,
  IdCard,
  MapPin,
  PanelLeftClose,
  PanelLeftOpen,
} from 'lucide-vue-next';
import PageHeader from './ui/PageHeader.vue';
import UiIconButton from './ui/UiIconButton.vue';
import InlineAlert from './ui/InlineAlert.vue';
import CampusRail, { type RailItem } from './campus/CampusRail.vue';
import CampusNotices from './campus/CampusNotices.vue';
import CampusLms from './campus/CampusLms.vue';
import CampusTodos from './campus/CampusTodos.vue';
import CampusLocalLibrary from './campus/CampusLocalLibrary.vue';
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

/**
 * 畅行校园 workspace (LAYOUT §2.1, §3.3): a section rail on the left and the
 * selected section on the right. Sections the backend cannot serve stay in the
 * rail, disabled with their reason, instead of disappearing.
 */
const props = withDefaults(
  defineProps<{
    name: string;
    accessMode: 'direct' | 'webvpn' | null;
    directSession?: boolean;
    railCollapsed?: boolean;
  }>(),
  { directSession: false, railCollapsed: false },
);
const emit = defineEmits<{ 'update-rail-collapsed': [collapsed: boolean]; 'direct-session': [] }>();

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
  localLibrary: CampusLocalLibrary,
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

const items: RailItem[] = [
  { id: 'todo', title: '待办', icon: ClipboardList },
  { id: 'notices', title: '通知', icon: Bell },
  { id: 'lms', title: '思源学堂', icon: BookOpen, group: '学习' },
  { id: 'localLibrary', title: '资料库', icon: FolderArchive, group: '学习' },
  { id: 'courseFiles', title: '课程资料', icon: FolderOpen, group: '学习' },
  { id: 'grades', title: '成绩', icon: BarChart3, group: '学业' },
  { id: 'exams', title: '考试', icon: ClipboardList, group: '学业' },
  { id: 'evaluations', title: '评教', icon: ClipboardCheck, group: '学业' },
  { id: 'rooms', title: '空闲教室', icon: DoorOpen, group: '学业' },
  { id: 'courses', title: '全校课程', icon: BookOpenText, group: '学业' },
  { id: 'calendar', title: '校历', icon: CalendarDays, group: '学业' },
  { id: 'card', title: '校园卡', icon: CreditCard, group: '生活' },
  { id: 'library', title: '图书馆', icon: BookMarked, group: '生活' },
  { id: 'venues', title: '场馆', icon: MapPin, group: '生活' },
  { id: 'attendance', title: '考勤', icon: CalendarCheck, group: '生活' },
  { id: 'fitness', title: '体测', icon: Activity, group: '生活' },
  { id: 'profile', title: '学籍档案', icon: IdCard, group: '生活' },
];
// Remember the last open section so the campus page never lands on an empty
// default. A disabled section is ignored, in case the list changed.
const LAST_SECTION = 'campus.lastSection';
const savedSection = (() => {
  try {
    const saved = localStorage.getItem(LAST_SECTION) ?? '';
    // The LMS course materials were briefly called 资料库; keep that choice.
    return saved === 'libraryFiles' ? 'courseFiles' : saved;
  } catch {
    return '';
  }
})();
if (
  savedSection &&
  views[savedSection] &&
  !items.find((item) => item.id === savedSection)?.disabled
)
  tool.value = savedSection;

const railItems = computed<RailItem[]>(() =>
  items.map((item) => ({
    ...item,
    disabled: item.disabled || false,
    reason: vpnBlocked(item.id) ? 'WebVPN 下不可用，需要先建立直连会话' : item.reason,
  })),
);

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
  if (items.find((item) => item.id === id)?.disabled) return;
  error.value = '';
  if (!(await openDirect(id))) return;
  tool.value = id;
  error.value = '';
  try {
    localStorage.setItem(LAST_SECTION, id);
  } catch {
    // A missing preference only costs the default section on the next launch.
  }
}

function toggleRail() {
  emit('update-rail-collapsed', !props.railCollapsed);
}
</script>

<template>
  <section class="desktop-page">
    <PageHeader :title="name">
      <template #actions>
        <UiIconButton
          :label="railCollapsed ? '展开分区导航' : '收起分区导航'"
          :title="railCollapsed ? '展开分区导航' : '收起分区导航'"
          @click="toggleRail"
        >
          <PanelLeftOpen v-if="railCollapsed" :size="16" />
          <PanelLeftClose v-else :size="16" />
        </UiIconButton>
      </template>
    </PageHeader>

    <div class="campus-workspace">
      <CampusRail
        :items="railItems"
        :model-value="tool"
        :collapsed="railCollapsed"
        @update:model-value="select"
      />
      <div class="campus-workspace-main">
        <InlineAlert v-if="error" tone="error" :title="error" />
        <component :is="views[tool]" />
      </div>
    </div>
  </section>
</template>

<style scoped>
.campus-workspace {
  display: flex;
  align-items: flex-start;
  gap: 24px;
}
.campus-workspace-main {
  flex: 1;
  min-width: 0;
}
</style>
