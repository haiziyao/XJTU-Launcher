<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { CalendarDays, ExternalLink } from 'lucide-vue-next';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import { bridge } from '../../../shared/bridge';
import { applyCached } from '../../../shared/cache';
import { currentAcademicStart, updatedInfo } from '../../../shared/domain';
import type { CalendarList } from '../../../types';

/**
 * 校历 (layout v2): the published calendar images for each academic year, listed
 * on the left and previewed in the centre. The backend copies each image into
 * the local cache and returns it as a data URL, so the picture is read from
 * disk instead of the school site; the cache is never bundled into the app.
 */
const calendar = ref<CalendarList | null>(null);
const selectedIndex = ref(0);
const loading = ref(false);
const loadedAt = ref<number | null>(null);
const error = ref('');
const image = ref('');
const imageLoading = ref(false);
const imageError = ref('');
let imageRequest = 0;

const academicStart = currentAcademicStart();
const rows = computed(() => calendar.value?.rows ?? []);
const activeIndex = computed(() => (rows.value[selectedIndex.value] ? selectedIndex.value : 0));
const selected = computed(() => rows.value[activeIndex.value] ?? null);
const title = computed(() => (selected.value?.year ? `${selected.value.year} 学年校历` : '校历'));

function currentTag(year: string | null) {
  return year && year.includes(String(academicStart)) ? '当前' : '';
}

async function load(force = false) {
  loading.value = true;
  error.value = '';
  try {
    await applyCached<CalendarList>('calendar', undefined, (value) => (calendar.value = value));
    calendar.value = (await bridge.schoolQuery(
      { type: 'calendar' },
      'calendar',
      force,
    )) as CalendarList;
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[campus] calendar', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function loadImage() {
  const entry = selected.value;
  const request = ++imageRequest;
  image.value = '';
  imageError.value = '';
  if (!entry) {
    imageLoading.value = false;
    return;
  }
  imageLoading.value = true;
  try {
    const data = await bridge.calendarImage(entry.url);
    if (request === imageRequest) image.value = data;
  } catch (e) {
    if (request === imageRequest) {
      console.error('[campus] calendar image', e);
      imageError.value = e instanceof Error ? e.message : String(e);
    }
  } finally {
    if (request === imageRequest) imageLoading.value = false;
  }
}

function open(url: string) {
  void bridge.openUrl(url);
}

watch(() => selected.value?.url, loadImage);

onMounted(() => void load());
</script>

<template>
  <CampusHeaderSlot slot="meta">
    <span v-if="loadedAt">更新于 {{ updatedInfo(loadedAt) }}</span>
  </CampusHeaderSlot>
  <CampusHeaderSlot slot="tools">
    <button type="button" class="cv-btn" :disabled="loading" @click="load(true)">刷新</button>
    <button
      type="button"
      class="cv-btn primary"
      :disabled="!selected"
      @click="selected && open(selected.url)"
    >
      <ExternalLink :size="13" />在浏览器打开
    </button>
  </CampusHeaderSlot>

  <div class="cv-cols lc">
    <div class="cv-panel cv-list">
      <div class="cv-lh">学年</div>
      <button
        v-for="(row, index) in rows"
        :key="index"
        type="button"
        class="cv-item plain"
        :class="{ on: activeIndex === index }"
        @click="selectedIndex = index"
      >
        <b>{{ row.year || '未标注学年' }}</b>
        <span v-if="currentTag(row.year)" class="cv-n">{{ currentTag(row.year) }}</span>
      </button>
    </div>

    <div class="cv-center">
      <InlineAlert v-if="error" tone="error" :title="error" />
      <div class="cv-ch">
        <b>{{ title }}</b>
        <span class="meta">教务处发布 · 网页内容</span>
      </div>

      <div v-if="loading && !rows.length" class="cv-empty">
        <CalendarDays :size="22" /><b>正在读取校历…</b>
      </div>
      <div v-else-if="!rows.length" class="cv-empty">
        <CalendarDays :size="22" /><b>暂时没有校历图片</b>
      </div>
      <div v-else class="cv-panel cal-prev">
        <div v-if="image" class="cal-shot">
          <img :src="image" :alt="title" class="cal-img" />
        </div>
        <div v-else class="cal-paper">
          <span v-if="imageLoading">正在读取校历图片…</span>
          <span v-else>{{ imageError || '暂时无法显示校历图片' }}</span>
        </div>
        <div class="cal-actions">
          <button type="button" class="cv-btn" @click="selected && open(selected.url)">
            <ExternalLink :size="13" />在浏览器打开
          </button>
          <span class="cv-muted cal-note">图片自动保存到本机缓存，不随安装包发布。</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.cal-prev {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
  padding: 12px 14px;
  overflow: auto;
}
.cal-paper {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  min-height: 280px;
  padding: 20px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface-subtle);
  color: var(--text-tertiary);
  font-size: 12px;
  text-align: center;
}
.cal-shot {
  display: flex;
  flex: 1;
  min-height: 280px;
  justify-content: center;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface);
}
.cal-img {
  display: block;
  width: 100%;
  height: auto;
  object-fit: contain;
  align-self: flex-start;
}
.cal-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}
.cal-note {
  font-size: 11.5px;
}
</style>
