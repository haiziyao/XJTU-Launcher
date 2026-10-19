<script setup lang="ts">
import { MessagesSquare } from 'lucide-vue-next';
import PageHeader from './ui/PageHeader.vue';
import EmptyState from './ui/EmptyState.vue';
</script>
<template>
  <section class="desktop-page">
    <PageHeader title="社区" />
    <EmptyState
      :icon="MessagesSquare"
      title="尚未接入社区"
      description="接入 Web 论坛后，在这里查看社区内容。"
    />
  </section>
</template>
