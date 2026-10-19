<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { GraduationCap, LoaderCircle, MoreHorizontal, Pencil } from 'lucide-vue-next';
import { Menu } from '@tauri-apps/api/menu';
import { LogicalPosition } from '@tauri-apps/api/dpi';
import { ask } from '@tauri-apps/plugin-dialog';
import { bridge } from '../bridge';
import type { CampusAccount, CampusAccountList } from '../types';
import PageHeader from './ui/PageHeader.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import EmptyState from './ui/EmptyState.vue';
import UiAvatar from './ui/UiAvatar.vue';

const emit = defineEmits<{ changed: [] }>();
const book = ref<CampusAccountList>({ activeId: null, accounts: [] });
const busy = ref(false);
const loading = ref(true);
const error = ref('');
let menu: Menu | undefined;
let disposed = false;
async function load() {
  const accounts = await bridge.accounts();
  if (!disposed) book.value = accounts;
}
async function run(action: () => Promise<unknown>) {
  if (busy.value) return;
  busy.value = true;
  error.value = '';
  try {
    await action();
    await load();
    emit('changed');
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = false;
  }
}
function changeAvatar(id: string) {
  return run(() => bridge.setAccountAvatar(id));
}
function switchAccount(id: string) {
  return run(() => bridge.switchAccount(id));
}
async function more(event: MouseEvent, account: CampusAccount) {
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  try {
    await menu?.close();
    menu = await Menu.new({
      items: [
        {
          id: 'avatar',
          text: '更换头像',
          action: () => void run(() => bridge.setAccountAvatar(account.id)),
        },
        {
          id: 'clear-avatar',
          text: '恢复默认头像',
          enabled: !!account.avatar,
          action: () => void run(() => bridge.clearAccountAvatar(account.id)),
        },
        { item: 'Separator' },
        {
          id: 'remove',
          text: '移除账号',
          action: () =>
            void run(async () => {
              if (
                await ask('移除此设备保存的账号和会话？学校账号不会被注销。', {
                  title: '移除账号',
                  kind: 'warning',
                  okLabel: '移除',
                  cancelLabel: '取消',
                })
              )
                await bridge.removeAccount(account.id);
            }),
        },
      ],
    });
    await menu.popup(new LogicalPosition(rect.left, rect.bottom));
  } catch (e) {
    error.value = String(e);
  }
}
onMounted(async () => {
  try {
    await load();
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
});
onBeforeUnmount(() => {
  disposed = true;
  void menu?.close();
});
</script>
<template>
  <section class="desktop-page narrow-page">
    <PageHeader title="校园账号" description="账号与会话只保存在本机，学校账号不会被注销" />
    <Transition name="drop">
      <p v-if="error" class="notice" role="alert">{{ error }}</p>
    </Transition>
    <SettingGroup title="已保存的账号">
      <EmptyState v-if="loading" compact :icon="LoaderCircle" busy title="正在读取账号…" />
      <EmptyState
        v-else-if="!book.accounts.length"
        compact
        :icon="GraduationCap"
        title="尚未添加校园账号"
        description="添加账号后即可同步课表等校园数据"
      />
      <TransitionGroup v-else name="list">
        <SettingRow v-for="account in book.accounts" :key="account.id">
          <template #leading>
            <button
              class="account-avatar"
              title="更换头像"
              :aria-label="'更换' + (account.name || account.username) + '的头像'"
              :disabled="busy"
              @click="changeAvatar(account.id)"
            >
              <UiAvatar :src="account.avatar" :size="36" />
              <span class="account-avatar-edit" aria-hidden="true"><Pencil :size="10" /></span>
            </button>
          </template>
          <template #title>
            {{ account.name || account.username }}
            <span v-if="account.id === book.activeId" class="badge accent">当前</span>
          </template>
          <template #description>
            {{ account.studentId || account.username }} ·
            {{ account.degree === 'undergraduate' ? '本科生' : '研究生' }}
            <template v-if="account.accessMode === 'webvpn'"> · WebVPN</template>
          </template>
          <button
            v-if="account.id !== book.activeId"
            class="btn small"
            :disabled="busy"
            @click="switchAccount(account.id)"
          >
            切换
          </button>
          <button
            class="btn ghost icon small"
            :disabled="busy"
            :aria-label="'管理' + (account.name || account.username)"
            @click="more($event, account)"
          >
            <MoreHorizontal :size="16" />
          </button>
        </SettingRow>
      </TransitionGroup>
    </SettingGroup>
  </section>
</template>
