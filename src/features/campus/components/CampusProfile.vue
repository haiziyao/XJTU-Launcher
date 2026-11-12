<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import UiAvatar from '../../../components/ui/UiAvatar.vue';
import { bridge } from '../../../shared/bridge';
import { siteQuery } from '../campusSession';
import { applyCached } from '../../../shared/cache';
import type { ProfileInfo } from '../../../types';

/**
 * 学籍档案 (layout v2): the identity card on the left and the fixed set of
 * student fields as compact key/value groups on the right. The page is read
 * only; the profile service supplies every field shown here.
 */
withDefaults(defineProps<{ avatar?: string | null }>(), { avatar: null });
const profile = ref<ProfileInfo | null>(null);
const loading = ref(true);
const error = ref('');

const rows = computed(() => {
  const value = profile.value;
  if (!value) return [];
  return [
    { label: '校区', text: value.campus },
    { label: '书院', text: value.academy },
    { label: '学院', text: value.department },
    { label: '专业', text: value.major },
    { label: '班级', text: value.className },
    { label: '年级', text: value.grade },
    { label: '性别', text: value.sex },
    { label: '出生日期', text: value.birthdate },
    { label: '学制', text: value.schoolingLen },
    { label: '入学时间', text: value.enterSchoolDate },
    { label: '卡号', text: value.cardId },
  ].filter((row) => row.text);
});

const GROUPS = [
  { title: '学籍', labels: ['校区', '书院', '年级', '学制', '入学时间'] },
  { title: '院系', labels: ['学院', '专业', '班级'] },
  { title: '个人', labels: ['性别', '出生日期', '卡号'] },
];
const groups = computed(() =>
  GROUPS.map((group) => ({
    title: group.title,
    rows: rows.value.filter((row) => group.labels.includes(row.label)),
  })).filter((group) => group.rows.length),
);

const contacts = computed(() => {
  const value = profile.value;
  if (!value) return [];
  return [
    { label: '班主任', text: value.classTeacher, phone: value.classTeacherPhone },
    { label: '辅导员', text: value.counselor, phone: value.counselorPhone },
    { label: '辅导员办公室', text: value.counselorOffice, phone: '' },
  ].filter((row) => row.text || row.phone);
});

const meta = computed(() =>
  profile.value ? [profile.value.name, profile.value.studentNo].filter(Boolean).join(' · ') : '',
);

async function load() {
  await applyCached<ProfileInfo>('profile', undefined, (value) => (profile.value = value));
  loading.value = true;
  error.value = '';
  try {
    profile.value = await siteQuery<ProfileInfo>(
      { type: 'profile' },
      'profile',
      () => bridge.refreshProfileSession(),
      'profile',
    );
  } catch (e) {
    console.error('[campus] profile', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(load);
</script>

<template>
  <CampusHeaderSlot slot="meta">
    <span v-if="meta">{{ meta }}</span>
  </CampusHeaderSlot>
  <CampusHeaderSlot slot="tools">
    <button type="button" class="cv-btn" :disabled="loading" @click="load">刷新</button>
  </CampusHeaderSlot>

  <p v-if="error" class="notice" role="alert">{{ error }}</p>

  <div class="cv-cols lc">
    <div class="cv-panel profile-id">
      <UiAvatar
        :src="profile?.pictureUrl || null"
        :fallback="avatar"
        :size="84"
        :alt="profile?.name || '学生头像'"
      />
      <b>{{ profile?.name || (loading ? '正在读取…' : '姓名未提供') }}</b>
      <span v-if="profile?.studentNo" class="no">{{ profile.studentNo }}</span>
      <span v-if="profile?.major || profile?.className" class="tag">
        {{ [profile?.major, profile?.className].filter(Boolean).join(' · ') }}
      </span>
      <span v-if="profile?.campus || profile?.grade" class="tag">
        {{ [profile?.campus, profile?.grade].filter(Boolean).join(' · ') }}
      </span>
    </div>

    <div class="cv-center">
      <div v-if="loading && !profile" class="cv-empty">正在读取学籍档案…</div>
      <div v-else-if="groups.length || contacts.length" class="profile-groups">
        <section v-for="group in groups" :key="group.title" class="cv-panel profile-g">
          <h4>{{ group.title }}</h4>
          <dl class="cv-kv">
            <template v-for="row in group.rows" :key="row.label">
              <dt>{{ row.label }}</dt>
              <dd>{{ row.text }}</dd>
            </template>
          </dl>
        </section>
        <section v-if="contacts.length" class="cv-panel profile-g">
          <h4>班主任与辅导员</h4>
          <dl class="cv-kv">
            <template v-for="row in contacts" :key="row.label">
              <dt>{{ row.label }}</dt>
              <dd>
                <template v-if="row.text">{{ row.text }}</template>
                <span v-if="row.phone" class="cv-muted cv-num">
                  <template v-if="row.text"> · </template>{{ row.phone }}
                </span>
              </dd>
            </template>
          </dl>
        </section>
      </div>
      <div v-else class="cv-placeholder">没有可显示的学籍信息。</div>
    </div>
  </div>
</template>

<style scoped>
.profile-id {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 18px 16px;
  text-align: center;
}
.profile-id b {
  color: var(--text);
  font-size: 16px;
  font-weight: 600;
}
.profile-id .no {
  color: var(--text-secondary);
  font-size: 12.5px;
  font-variant-numeric: tabular-nums;
}
.profile-id .tag {
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.profile-groups {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  align-content: start;
  min-height: 0;
  overflow: auto;
  overscroll-behavior: contain;
}
.profile-g {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 14px;
}
.profile-g h4 {
  margin: 0;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
}
.profile-g .cv-kv {
  grid-template-columns: 72px 1fr;
  row-gap: 7px;
  font-size: 12.5px;
}
</style>
