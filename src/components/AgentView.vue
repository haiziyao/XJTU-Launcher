<script setup lang="ts">
import { Bot, Plug } from 'lucide-vue-next';
import PageHeader from './ui/PageHeader.vue';
import EmptyState from './ui/EmptyState.vue';
const emit = defineEmits<{ models: []; mcp: [] }>();
</script>
<template>
  <section class="desktop-page">
    <PageHeader title="Agent">
      <template #actions>
        <button class="btn" @click="emit('mcp')"><Plug :size="14" />MCP 管理</button>
      </template>
    </PageHeader>
    <EmptyState
      :icon="Bot"
      title="Agent 尚未接入"
      description="可以先配置模型服务和 MCP 工具，接入后直接使用。"
    >
      <button class="btn" @click="emit('models')">模型服务设置</button>
    </EmptyState>
  </section>
</template>
