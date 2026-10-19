<script setup lang="ts">
import { ref } from 'vue';
import EditableText from './EditableText.vue';
import { Camera, GraduationCap, ImagePlus } from 'lucide-vue-next';
import type { AccountStatus, AppConfig } from '../types';
import { bridge } from '../bridge';
import PageHeader from './ui/PageHeader.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import UiAvatar from './ui/UiAvatar.vue';
import IconTile from './ui/IconTile.vue';
const props = defineProps<{ config: AppConfig; account: AccountStatus; saving: boolean }>();
const emit = defineEmits<{ update: [config: AppConfig]; accounts: [] }>();
const error = ref('');
const importing = ref(false);
async function choose(kind: 'avatar' | 'wallpaper') {
  importing.value = true;
  error.value = '';
  try {
    const image = await bridge.personalImage(kind);
    if (image) emit('update', { ...props.config, [kind]: image });
  } catch (e) {
    error.value = String(e);
  } finally {
    importing.value = false;
  }
}
</script>
<template>
  <section class="desktop-page narrow-page">
    <PageHeader title="个人" description="本机个人资料与主页外观" />
    <Transition name="drop">
      <p v-if="error" class="notice" role="alert">{{ error }}</p>
    </Transition>

    <div class="profile-card">
      <button
        class="profile-avatar"
        title="更换个人头像"
        aria-label="更换个人头像"
        :disabled="saving || importing"
        @click="choose('avatar')"
      >
        <UiAvatar :src="config.avatar || account.avatar" :size="64" alt="个人头像" />
        <span class="profile-avatar-overlay" aria-hidden="true"><Camera :size="18" /></span>
      </button>
      <div class="profile-copy">
        <strong>{{
          config.nickname || account.name || (account.connected ? '校园账号' : '未登录')
        }}</strong>
        <small>{{ account.studentId || '本机个人资料' }}</small>
      </div>
      <div class="row-actions">
        <button
          v-if="config.avatar"
          class="btn ghost small"
          :disabled="saving || importing"
          @click="emit('update', { ...config, avatar: null })"
        >
          恢复默认
        </button>
        <button class="btn small" :disabled="saving || importing" @click="choose('avatar')">
          更换头像
        </button>
      </div>
    </div>

    <SettingGroup title="资料">
      <SettingRow title="显示名称" description="显示在侧栏与主页问候中">
        <EditableText
          :value="config.nickname"
          :placeholder="account.name || '使用校园账号姓名'"
          label="显示名称"
          :disabled="saving"
          :maxlength="40"
          @save="emit('update', { ...config, nickname: $event })"
        />
      </SettingRow>
    </SettingGroup>

    <SettingGroup title="主页壁纸">
      <div class="wallpaper-row">
        <div class="wallpaper-preview" :class="{ empty: !config.wallpaper }">
          <Transition name="fade" mode="out-in">
            <img
              v-if="config.wallpaper"
              :key="config.wallpaper"
              :src="config.wallpaper"
              alt="当前主页壁纸"
            />
            <span v-else>默认背景</span>
          </Transition>
        </div>
        <div class="wallpaper-copy">
          <strong>{{ config.wallpaper ? '自定义壁纸' : '默认背景' }}</strong>
          <small>壁纸只在主页显示，卡片会自动加上半透明底色。</small>
          <div class="row-actions">
            <button class="btn small" :disabled="saving || importing" @click="choose('wallpaper')">
              <ImagePlus :size="14" />选择图片
            </button>
            <button
              v-if="config.wallpaper"
              class="btn ghost small"
              :disabled="saving || importing"
              @click="emit('update', { ...config, wallpaper: null })"
            >
              移除
            </button>
          </div>
        </div>
      </div>
    </SettingGroup>

    <SettingGroup title="校园身份">
      <SettingRow
        clickable
        title="校园账号"
        :description="account.connected ? '管理已保存的账号、头像与当前身份' : '尚未登录'"
        @select="emit('accounts')"
      >
        <template #leading>
          <IconTile tone="blue" :size="30"><GraduationCap :size="16" /></IconTile>
        </template>
        <span v-if="account.connected" class="badge success">已连接</span>
      </SettingRow>
    </SettingGroup>
  </section>
</template>
