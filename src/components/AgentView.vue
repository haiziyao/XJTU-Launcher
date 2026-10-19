<script setup lang="ts">
import { Bot } from 'lucide-vue-next';
import PageHeader from './ui/PageHeader.vue';
import EmptyState from './ui/EmptyState.vue';
const emit = defineEmits<{ models: [] }>();
</script>
<template>
  <section class="desktop-page">
    <PageHeader title="Agent" />
    <EmptyState
      :icon="Bot"
      title="Agent 尚未接入"
      description="可以先配置模型服务，接入后直接使用。"
    >
      <button class="btn" @click="emit('models')">模型服务设置</button>
    </EmptyState>
  </section>
</template>
