<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import SettingGroup from '../ui/SettingGroup.vue';
import SettingRow from '../ui/SettingRow.vue';
import EmptyState from '../ui/EmptyState.vue';
import UiAvatar from '../ui/UiAvatar.vue';
import { bridge } from '../../bridge';
import { siteQuery } from '../../campusSession';
import { applyCached } from '../../cache';
import type { ProfileInfo } from '../../types';

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

const contacts = computed(() => {
  const value = profile.value;
  if (!value) return [];
  return [
    { label: '班主任', text: value.classTeacher, phone: value.classTeacherPhone },
    { label: '辅导员', text: value.counselor, phone: value.counselorPhone },
    { label: '辅导员办公室', text: value.counselorOffice, phone: '' },
  ].filter((row) => row.text || row.phone);
});

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
  <p v-if="error" class="notice" role="alert">{{ error }}</p>
  <SettingGroup title="学籍档案">
    <EmptyState v-if="loading && !profile" compact title="正在读取学籍档案…" />
    <SettingRow
      v-else-if="profile"
      :title="profile.name || '姓名未提供'"
      :description="[profile.studentNo, profile.major].filter(Boolean).join(' · ')"
    >
      <template #leading>
        <UiAvatar :src="profile.pictureUrl || null" :size="40" :alt="profile.name || '学生头像'" />
      </template>
    </SettingRow>
  </SettingGroup>
  <SettingGroup v-if="rows.length" title="基本信息">
    <SettingRow v-for="row in rows" :key="row.label" :title="row.label">
      <span class="setting-value">{{ row.text }}</span>
    </SettingRow>
  </SettingGroup>
  <SettingGroup v-if="contacts.length" title="班主任与辅导员">
    <SettingRow
      v-for="row in contacts"
      :key="row.label"
      :title="row.label"
      :description="[row.text, row.phone].filter(Boolean).join(' · ')"
    />
  </SettingGroup>
</template>
