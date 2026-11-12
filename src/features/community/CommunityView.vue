<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import type { CSSProperties } from 'vue';
import { ArrowLeft, Search, Star } from 'lucide-vue-next';
import { bridge } from '../../shared/bridge';
import {
  AGENT_REASON_TEXT,
  COMMUNITY_FAVORITES_KEY,
  COMMUNITY_SECTIONS,
  parseFavoriteIds,
  searchSections,
  toggleFavoriteId,
} from './community';
import type { CommunitySection, CommunitySectionId } from './community';
import ResourceSitesView from './ResourceSitesView.vue';
import type { AgentSearchReason, AgentSearchStatus, ResourceCatalog } from '../../types';
import PageHeader from '../../components/ui/PageHeader.vue';
import UiButton from '../../components/ui/UiButton.vue';
import UiIconButton from '../../components/ui/UiIconButton.vue';

type SearchMode = 'normal' | 'agent';
type SearchPanel =
  | { kind: 'normal'; results: CommunitySection[] }
  | { kind: 'agent'; answer: string }
  | { kind: 'error'; message: string };

/** A successful reachability probe is reused for a minute. */
const PROBE_TTL = 60_000;

/** Compact favorites tile measurements, matched by the reference sheet. */
const FAVORITE_TILE = 84;
const FAVORITE_MIN_GAP = 8;
const FAVORITE_MAX_WIDTH = 280;

const activeSection = ref<CommunitySection | null>(null);
const favorites = ref<CommunitySectionId[]>([]);
const hoveredFavorite = ref<CommunitySectionId | null>(null);
const favoriteRow = ref<HTMLElement | null>(null);
const favoriteOpenWidth = ref(FAVORITE_MAX_WIDTH);
const resourceCatalog = ref<ResourceCatalog | null>(null);
const resourceLoading = ref(false);
const resourceError = ref('');
let resourceRequested = false;
const query = ref('');
const mode = ref<SearchMode>('normal');
const panel = ref<SearchPanel | null>(null);
const validation = ref('');
const agentAvailable = ref(false);
const agentReason = ref<AgentSearchReason | null>(null);
const agentChecking = ref(false);
const agentBusy = ref(false);
let probeSucceededAt = 0;
/** Ignores an Agent answer that finishes after the user changed the search. */
let searchGeneration = 0;

const favoriteSections = computed(() =>
  favorites.value
    .map((id) => COMMUNITY_SECTIONS.find((section) => section.id === id))
    .filter((section): section is CommunitySection => Boolean(section)),
);
const modeLabel = computed(() => (mode.value === 'agent' ? 'Agent' : '普通'));
const agentDisabled = computed(() => !agentAvailable.value || agentChecking.value);
const agentStatusText = computed(() => {
  if (agentChecking.value && !agentBusy.value) return '正在检查 Agent 服务…';
  return agentReason.value ? AGENT_REASON_TEXT[agentReason.value] : '';
});

function sectionStyle(section: CommunitySection): CSSProperties {
  return {
    '--section-accent': section.accent,
    '--section-soft': section.accentSoft,
  };
}

function favoriteStyle(section: CommunitySection): CSSProperties {
  return { ...sectionStyle(section), '--favorite-open-width': `${favoriteOpenWidth.value}px` };
}

function readFavorites() {
  try {
    favorites.value = parseFavoriteIds(localStorage.getItem(COMMUNITY_FAVORITES_KEY));
  } catch {
    favorites.value = [];
  }
}

function saveFavorites() {
  try {
    localStorage.setItem(COMMUNITY_FAVORITES_KEY, JSON.stringify(favorites.value));
  } catch {
    // Favorites only affect this device; a full storage quota is not an error.
  }
}

function toggleFavorite(id: CommunitySectionId) {
  favorites.value = toggleFavoriteId(favorites.value, id);
  saveFavorites();
  if (hoveredFavorite.value && !favorites.value.includes(hoveredFavorite.value)) {
    hoveredFavorite.value = null;
  }
}

/**
 * Expands one tile up to 280px, but never wider than the room the row has once
 * every compact tile and the minimum gaps are accounted for.
 */
function openFavorite(id: CommunitySectionId) {
  const count = favorites.value.length;
  const compactWidth = count * FAVORITE_TILE + Math.max(0, count - 1) * FAVORITE_MIN_GAP;
  const available = favoriteRow.value?.clientWidth ?? 0;
  const width = Math.max(
    FAVORITE_TILE,
    Math.min(FAVORITE_MAX_WIDTH, available - compactWidth + FAVORITE_TILE),
  );
  favoriteOpenWidth.value = width;
  hoveredFavorite.value = width > FAVORITE_TILE ? id : null;
}

function applyAgentStatus(status: AgentSearchStatus) {
  agentAvailable.value = status.available;
  agentReason.value = status.available ? null : status.reason;
}

async function fetchAgentStatus(probe: boolean): Promise<boolean> {
  agentChecking.value = true;
  try {
    const status = await bridge.agentSearchStatus(probe);
    applyAgentStatus(status);
    if (status.available && probe) probeSucceededAt = Date.now();
    return status.available;
  } catch {
    agentAvailable.value = false;
    agentReason.value = 'unreachable';
    return false;
  } finally {
    agentChecking.value = false;
  }
}

async function ensureAgentAvailable(): Promise<boolean> {
  // Local checks on mount never stand in for the first reachability probe.
  if (agentAvailable.value && Date.now() - probeSucceededAt < PROBE_TTL) return true;
  return fetchAgentStatus(true);
}

async function toggleMode() {
  if (mode.value === 'agent') {
    searchGeneration += 1;
    mode.value = 'normal';
    return;
  }
  if (agentDisabled.value) return;
  if (!(await ensureAgentAvailable())) return;
  mode.value = 'agent';
  panel.value = null;
  validation.value = '';
}

function submitSearch() {
  const value = query.value.trim();
  if (!value) {
    searchGeneration += 1;
    validation.value = '请输入搜索内容';
    panel.value = null;
    return;
  }
  validation.value = '';
  if (mode.value === 'agent') {
    void runAgentSearch(value);
  } else {
    searchGeneration += 1;
    panel.value = { kind: 'normal', results: searchSections(value) };
  }
}

function errorText(error: unknown, fallback = 'Agent 搜索失败'): string {
  if (typeof error === 'string' && error.trim()) return error.trim();
  if (error instanceof Error && error.message) return error.message;
  return fallback;
}

async function runAgentSearch(value: string) {
  if (agentBusy.value) return;
  agentBusy.value = true;
  const generation = ++searchGeneration;
  try {
    const available = await ensureAgentAvailable();
    if (generation !== searchGeneration) return;
    if (!available) {
      mode.value = 'normal';
      return;
    }
    const response = await bridge.communityAgentSearch(value);
    if (generation !== searchGeneration) return;
    panel.value = { kind: 'agent', answer: response.answer };
  } catch (error) {
    if (generation !== searchGeneration) return;
    // A failed Agent answer is not retried automatically; drop back to local
    // search and keep the reason visible beside the mode toggle.
    agentAvailable.value = false;
    agentReason.value = 'unreachable';
    mode.value = 'normal';
    panel.value = { kind: 'error', message: errorText(error) };
  } finally {
    agentBusy.value = false;
  }
}

async function loadResourceSites() {
  if (resourceRequested) return;
  resourceRequested = true;
  resourceLoading.value = true;
  resourceError.value = '';
  try {
    resourceCatalog.value = await bridge.resourceSites();
  } catch (error) {
    // Retry on the next entry so a transient read failure is not sticky.
    resourceRequested = false;
    resourceError.value = errorText(error, '无法读取资源站点');
  } finally {
    resourceLoading.value = false;
  }
}

function openSection(section: CommunitySection) {
  activeSection.value = section;
  if (section.id === 'links') void loadResourceSites();
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape' && activeSection.value) activeSection.value = null;
}

onMounted(() => {
  readFavorites();
  // Local checks only; opening the page must not request the model service.
  void fetchAgentStatus(false);
  window.addEventListener('keydown', onKeydown);
});
onBeforeUnmount(() => {
  searchGeneration += 1;
  window.removeEventListener('keydown', onKeydown);
});
</script>

<template>
  <ResourceSitesView
    v-if="activeSection?.id === 'links'"
    :catalog="resourceCatalog"
    :loading="resourceLoading"
    :error="resourceError"
    @back="activeSection = null"
  />

  <section v-else-if="activeSection" class="desktop-page community-page">
    <UiButton variant="ghost" class="back-button" @click="activeSection = null">
      <ArrowLeft :size="15" />返回社区广场
    </UiButton>
    <div class="community-placeholder" :style="sectionStyle(activeSection)">
      <span class="community-placeholder-icon" aria-hidden="true">
        <component :is="activeSection.icon" :size="26" :stroke-width="1.7" />
      </span>
      <h1>{{ activeSection.title }}</h1>
      <p class="community-placeholder-description">{{ activeSection.description }}</p>
      <p class="community-placeholder-note">该板块正在开发</p>
    </div>
  </section>

  <section v-else class="desktop-page community-page">
    <PageHeader title="社区广场" />

    <section class="community-block" aria-label="我的收藏">
      <h2 class="community-block-label">我的收藏</h2>
      <div
        v-if="favoriteSections.length"
        ref="favoriteRow"
        class="favorite-row"
        :class="{ 'has-open': hoveredFavorite }"
      >
        <div
          v-for="section in favoriteSections"
          :key="section.id"
          class="favorite-tile"
          :class="{ 'is-open': hoveredFavorite === section.id }"
          :style="favoriteStyle(section)"
          role="button"
          tabindex="0"
          :aria-label="`${section.title}，已收藏`"
          @click="openSection(section)"
          @keydown.enter.prevent="openSection(section)"
          @keydown.space.prevent="openSection(section)"
          @mouseenter="openFavorite(section.id)"
          @mouseleave="hoveredFavorite = null"
          @focusin="openFavorite(section.id)"
          @focusout="hoveredFavorite = null"
        >
          <span class="favorite-badge" aria-hidden="true">
            <component :is="section.icon" :size="19" :stroke-width="1.8" />
          </span>
          <span class="favorite-name">{{ section.title }}</span>
          <span class="favorite-desc">{{ section.description }}</span>
          <button
            type="button"
            class="community-star is-on favorite-star"
            :aria-label="`取消收藏 ${section.title}`"
            @click.stop="toggleFavorite(section.id)"
          >
            <Star :size="13" fill="currentColor" />
          </button>
        </div>
      </div>
      <p v-else class="favorite-empty">
        还没有收藏。点击下方板块右上角的星标，收藏后会出现在这里。
      </p>
    </section>

    <section class="community-block" aria-label="全部板块">
      <h2 class="community-block-label">全部板块</h2>
      <nav class="community-grid">
        <div
          v-for="section in COMMUNITY_SECTIONS"
          :key="section.id"
          class="community-card-shell"
          :style="sectionStyle(section)"
        >
          <button type="button" class="community-card" @click="openSection(section)">
            <span class="community-card-body">
              <span class="community-card-heading">
                <span class="community-card-icon" aria-hidden="true">
                  <component :is="section.icon" :size="15" :stroke-width="1.9" />
                </span>
                <strong>{{ section.title }}</strong>
              </span>
              <span class="community-card-description">{{ section.description }}</span>
            </span>
            <span class="community-media" aria-hidden="true">
              <span class="community-media-layer is-a">
                <span class="community-media-glow"></span>
                <component
                  :is="section.icon"
                  :size="38"
                  :stroke-width="1.25"
                  class="community-media-icon"
                />
              </span>
              <span class="community-media-layer is-b">
                <span class="community-media-glow"></span>
                <component
                  :is="section.icon"
                  :size="38"
                  :stroke-width="1.25"
                  class="community-media-icon"
                />
              </span>
            </span>
          </button>
          <button
            type="button"
            class="community-star"
            :class="{ 'is-on': favorites.includes(section.id) }"
            :aria-label="`${favorites.includes(section.id) ? '取消收藏' : '收藏'} ${section.title}`"
            @click="toggleFavorite(section.id)"
          >
            <Star :size="14" :fill="favorites.includes(section.id) ? 'currentColor' : 'none'" />
          </button>
        </div>
      </nav>
    </section>

    <section class="community-search" aria-label="社区搜索">
      <form class="community-search-form" role="search" @submit.prevent="submitSearch">
        <input
          v-model="query"
          class="community-search-input"
          type="text"
          autocomplete="off"
          spellcheck="false"
          placeholder="搜索社区…"
          aria-label="搜索社区板块"
        />
        <UiIconButton
          label="搜索"
          variant="primary"
          size="md"
          class="community-search-submit"
          @click="submitSearch"
        >
          <Search :size="15" />
        </UiIconButton>
        <UiButton
          type="button"
          size="sm"
          :variant="mode === 'agent' ? 'tinted' : 'ghost'"
          class="community-mode-toggle"
          :disabled="agentDisabled"
          :aria-label="`搜索模式：${modeLabel}，点击切换`"
          :title="agentDisabled && agentStatusText ? agentStatusText : undefined"
          @click="toggleMode"
        >
          {{ modeLabel }}
        </UiButton>
      </form>

      <p v-if="agentStatusText" class="community-agent-status" role="status">
        {{ agentStatusText }}
      </p>
      <p v-if="validation" class="community-validation" role="alert">{{ validation }}</p>
      <p v-if="agentBusy" class="community-agent-status" role="status">正在生成回答…</p>

      <div v-if="panel" class="community-results" aria-live="polite">
        <template v-if="panel.kind === 'normal'">
          <ul v-if="panel.results.length" class="community-result-list">
            <li v-for="result in panel.results" :key="result.id">
              <button
                type="button"
                class="community-result"
                :style="sectionStyle(result)"
                @click="openSection(result)"
              >
                <span class="community-result-icon" aria-hidden="true">
                  <component :is="result.icon" :size="15" :stroke-width="1.8" />
                </span>
                <span class="community-result-copy">
                  <strong>{{ result.title }}</strong>
                  <small>{{ result.description }}</small>
                </span>
              </button>
            </li>
          </ul>
          <p v-else class="community-empty">内容待开发</p>
        </template>

        <template v-else-if="panel.kind === 'agent'">
          <p class="community-agent-label">Agent 回答，未引用社区内容</p>
          <p class="community-agent-answer">{{ panel.answer }}</p>
        </template>

        <p v-else class="community-search-error" role="alert">{{ panel.message }}</p>
      </div>
    </section>
  </section>
</template>

<style scoped>
.community-page {
  padding-bottom: 8px;
}

.community-block + .community-block {
  margin-top: 26px;
}
.community-block-label {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 0 12px;
  color: var(--text-tertiary);
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.04em;
}

/* Favorites strip: compact squares keep their width, the hovered one grows,
   and the row's gaps close so both sides move outward. */
.favorite-row {
  --favorite-spring: cubic-bezier(0.28, 1.12, 0.42, 1);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 26px;
  min-height: 96px;
  padding: 6px 0 10px;
  transition: gap 340ms var(--ease-out);
}
.favorite-row.has-open {
  gap: 8px;
}

.favorite-tile {
  position: relative;
  flex: 0 0 auto;
  width: 84px;
  height: 84px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--panel, var(--surface));
  box-shadow: var(--shadow-xs);
  cursor: pointer;
  outline: none;
  transition:
    width 340ms var(--ease-out),
    border-color 160ms ease,
    box-shadow 160ms ease;
}
.favorite-tile.is-open {
  width: var(--favorite-open-width, 280px);
  border-color: var(--border-strong);
  box-shadow: var(--shadow-md);
}
.favorite-tile:focus-visible {
  border-color: var(--accent);
  box-shadow: var(--focus-ring);
}

/* One icon and one label morph from the stacked compact layout into the
   horizontal expanded layout. No second copy, no cross-fade. */
.favorite-badge {
  position: absolute;
  top: 25px;
  left: 20px;
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  border-radius: 10px;
  background: var(--section-soft);
  color: var(--section-accent);
  transform: translate(5px, -12px);
  transition: transform 340ms var(--favorite-spring);
}
.favorite-tile.is-open .favorite-badge {
  transform: none;
}

.favorite-name {
  position: absolute;
  top: 22px;
  left: 68px;
  overflow: hidden;
  max-width: 72px;
  font-size: 13.5px;
  font-weight: 600;
  line-height: 18px;
  text-overflow: ellipsis;
  white-space: nowrap;
  transform: translate(calc(-50% - 26px), 32px);
  transition:
    transform 340ms var(--favorite-spring),
    max-width 340ms var(--ease-out);
}
.favorite-tile.is-open .favorite-name {
  max-width: none;
  transform: none;
}

.favorite-desc {
  position: absolute;
  top: 45px;
  right: 20px;
  left: 68px;
  overflow: hidden;
  color: var(--text-secondary);
  font-size: 12.5px;
  line-height: 17px;
  opacity: 0;
  text-overflow: ellipsis;
  white-space: nowrap;
  transform: translateX(-10px);
  transition:
    opacity 120ms ease,
    transform 200ms var(--ease-out);
}
.favorite-tile.is-open .favorite-desc {
  opacity: 1;
  transform: none;
  transition:
    opacity 180ms ease 110ms,
    transform 300ms var(--ease-out) 110ms;
}

.favorite-empty {
  display: grid;
  place-items: center;
  min-height: 84px;
  margin: 0 0 8px;
  color: var(--text-tertiary);
  font-size: 12.5px;
}

/* Cards: 3 + 2 at full width, then 2 and 1 column as the window narrows. */
.community-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 14px;
}

.community-card-shell {
  position: relative;
  display: grid;
}

.community-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 92px;
  align-items: center;
  gap: 14px;
  width: 100%;
  height: 100%;
  min-height: 128px;
  padding: 16px 16px 16px 18px;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--panel, var(--surface));
  color: var(--text);
  text-align: left;
  box-shadow: var(--shadow-xs);
  transition:
    border-color var(--duration-fast) ease,
    box-shadow var(--duration-fast) ease,
    transform var(--duration-fast) var(--ease-out);
}

/* Star in the card's top-right corner, above the decorative media block. */
.community-star {
  position: absolute;
  top: 11px;
  right: 11px;
  z-index: 2;
  display: grid;
  place-items: center;
  width: 21px;
  height: 21px;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: rgba(255, 255, 255, 0.82);
  color: var(--text-tertiary);
  opacity: 0.75;
  backdrop-filter: blur(4px);
  transition:
    opacity 150ms ease,
    color 150ms ease,
    background 150ms ease,
    transform 150ms var(--ease-out);
}
.community-star:hover {
  background: #fff;
  color: var(--text-secondary);
  opacity: 1;
}
.community-star.is-on {
  color: #c98a12;
  opacity: 1;
}
.community-star:active {
  transform: scale(0.85);
}
.favorite-star {
  top: 4px;
  right: 4px;
}
.community-card:hover {
  border-color: var(--border-strong);
  box-shadow: var(--shadow-sm);
}
.community-card:active {
  transform: scale(0.985);
}

.community-card-body {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}
.community-card-heading {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
}
.community-card-icon {
  display: grid;
  place-items: center;
  flex-shrink: 0;
  width: 24px;
  height: 24px;
  border-radius: var(--radius-sm);
  background: var(--section-soft);
  color: var(--section-accent);
}
.community-card-description {
  color: var(--text-secondary);
  font-size: 12.5px;
  line-height: 1.5;
}

/* Decorative media block: two abstract layers cross-fade in place. Hover or
   keyboard focus freezes the current frame; reduced motion keeps one frame. */
.community-media {
  position: relative;
  display: block;
  height: 96px;
  overflow: hidden;
  border-radius: var(--radius);
  background: var(--section-soft);
  isolation: isolate;
}
.community-media-layer {
  position: absolute;
  inset: 0;
  opacity: 0;
  animation: community-media-cycle 10s ease-in-out infinite;
}
.community-media-layer.is-a {
  background:
    radial-gradient(120% 110% at 12% 8%, rgba(255, 255, 255, 0.75), transparent 48%),
    linear-gradient(140deg, var(--section-soft), var(--section-accent));
}
.community-media-layer.is-b {
  animation-delay: -5s;
  background:
    radial-gradient(120% 110% at 88% 92%, rgba(255, 255, 255, 0.7), transparent 52%),
    linear-gradient(320deg, var(--section-soft) 10%, var(--section-accent) 150%);
}
.community-media-glow {
  position: absolute;
  border: 1px solid rgba(255, 255, 255, 0.55);
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.14);
}
.is-a .community-media-glow {
  top: -22px;
  right: -18px;
  width: 68px;
  height: 68px;
}
.is-b .community-media-glow {
  bottom: -26px;
  left: -16px;
  width: 82px;
  height: 82px;
  background: rgba(255, 255, 255, 0.2);
}
.community-media-icon {
  position: absolute;
  color: rgba(255, 255, 255, 0.9);
}
.is-a .community-media-icon {
  right: 10px;
  bottom: 8px;
  transform: rotate(-8deg);
}
.is-b .community-media-icon {
  top: 10px;
  left: 10px;
  transform: rotate(7deg);
}
.community-card:hover .community-media-layer,
.community-card:focus-visible .community-media-layer {
  animation-play-state: paused;
}
@keyframes community-media-cycle {
  0%,
  42% {
    opacity: 1;
  }
  48%,
  92% {
    opacity: 0;
  }
  98%,
  100% {
    opacity: 1;
  }
}

/* Search: lower-middle of the page, not in the header. */
.community-search {
  display: flex;
  flex-direction: column;
  align-items: center;
  margin-top: 34px;
}
.community-search-form {
  display: flex;
  align-items: center;
  gap: 8px;
  width: min(100%, 560px);
  padding: 6px 8px 6px 14px;
  border: 1px solid var(--control-border);
  border-radius: var(--radius-lg);
  background: var(--control-bg);
  box-shadow: var(--shadow-sm);
  transition:
    border-color var(--duration-fast) ease,
    box-shadow var(--duration-fast) ease;
}
.community-search-form:focus-within {
  border-color: var(--accent);
  box-shadow: var(--focus-ring);
}
.community-search-input {
  flex: 1;
  min-width: 0;
  height: 30px;
  padding: 0;
  border: 0;
  outline: none;
  background: transparent;
  color: var(--text);
  font-size: 13px;
}
.community-search-input::placeholder {
  color: var(--text-tertiary);
}
.community-mode-toggle {
  flex-shrink: 0;
}

.community-agent-status,
.community-validation {
  margin-top: 8px;
  color: var(--text-tertiary);
  font-size: 12px;
  text-align: center;
}
/* Keep the unavailable reason next to the mode button it explains. */
.community-agent-status {
  width: min(100%, 560px);
  text-align: right;
}
.community-validation,
.community-search-error {
  color: var(--danger);
}

.community-results {
  width: min(100%, 560px);
  margin-top: 12px;
  padding: 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--panel, var(--surface));
  box-shadow: var(--shadow-sm);
}
.community-result-list {
  margin: 0;
  padding: 0;
  list-style: none;
}
.community-result-list li + li {
  border-top: 1px solid var(--divider);
}
.community-result {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 9px 10px;
  border: 0;
  border-radius: var(--radius);
  background: transparent;
  color: var(--text);
  text-align: left;
}
.community-result:hover,
.community-result:focus-visible {
  background: var(--surface-hover);
}
.community-result-icon {
  display: grid;
  place-items: center;
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  border-radius: var(--radius-sm);
  background: var(--section-soft);
  color: var(--section-accent);
}
.community-result-copy {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}
.community-result-copy strong {
  font-size: 13px;
  font-weight: 500;
}
.community-result-copy small {
  color: var(--text-tertiary);
  font-size: 12px;
}
.community-empty {
  padding: 14px 12px;
  color: var(--text-secondary);
  font-size: 12.5px;
  text-align: center;
}

.community-agent-label {
  margin: 2px 6px 6px;
  color: var(--text-tertiary);
  font-size: 12px;
}
.community-agent-answer {
  margin: 0;
  padding: 0 6px 6px;
  color: var(--text);
  font-size: 13px;
  line-height: 1.65;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.community-search-error {
  margin: 0;
  padding: 12px 10px;
  font-size: 12.5px;
  line-height: 1.55;
  overflow-wrap: anywhere;
}

/* Section placeholder: only the back action, heading and the pending note. */
.community-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  margin-top: 64px;
  text-align: center;
}
.community-placeholder-icon {
  display: grid;
  place-items: center;
  width: 52px;
  height: 52px;
  margin-bottom: 16px;
  border-radius: var(--radius-lg);
  background: var(--section-soft);
  color: var(--section-accent);
}
.community-placeholder h1 {
  font-size: 20px;
  font-weight: 600;
  letter-spacing: -0.01em;
}
.community-placeholder-description {
  margin-top: 6px;
  color: var(--text-secondary);
  font-size: 13px;
}
.community-placeholder-note {
  margin-top: 28px;
  padding: 12px 20px;
  border: 1px dashed var(--border-strong);
  border-radius: var(--radius);
  color: var(--text-tertiary);
  font-size: 12.5px;
}

@media (max-width: 980px) {
  .community-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
@media (max-width: 640px) {
  .community-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
@media (prefers-reduced-motion: reduce) {
  .community-card:active,
  .community-star:active {
    transform: none;
  }
  .favorite-row,
  .favorite-tile,
  .favorite-badge,
  .favorite-name,
  .favorite-desc {
    transition-duration: 0.01ms;
    transition-delay: 0ms;
  }
  .community-media-layer {
    animation: none;
  }
  .community-media-layer.is-a {
    opacity: 1;
  }
  .community-media-layer.is-b {
    opacity: 0;
  }
}
</style>
