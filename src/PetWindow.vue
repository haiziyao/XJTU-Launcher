<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { bridge } from './bridge';
import type { PetDesktopSettings, PetDesktopView, PetNotification } from './types';
import deepseekGirl from './assets/pets/deepseek-girl.webp';
import whalechan from './assets/pets/deepseek-whalechan.webp';
import gptDragon from './assets/pets/gpt-dragon-girl.webp';
import claude from './assets/pets/claude.webp';

const CHARACTERS = [
  { id: 'deepseek-girl', name: 'DeepSeek 娘', file: deepseekGirl, height: 2288 },
  { id: 'deepseek-whalechan', name: '鲸鱼娘', file: whalechan, height: 2288 },
  { id: 'gpt-dragon-girl', name: 'GPT 龙娘', file: gptDragon, height: 2288 },
  { id: 'claude', name: 'Claude', file: claude, height: 1872 },
] as const;

const LINES = [
  '今天也要好好上课呀',
  '记得喝水',
  '课表已经刷新啦',
  '有通知我会提醒你',
  '摸鱼也要适度哦',
  '下课记得吃饭',
];

const ROWS: Record<string, [number, number]> = {
  idle: [0, 200],
  waving: [3, 140],
  jumping: [4, 130],
};

const FALLBACK: PetDesktopSettings = {
  enabled: true,
  character: 'deepseek-girl',
  barStyle: 'glass-light',
  barHidden: false,
  bubble: true,
  sources: ['campus', 'agent', 'class', 'update'],
  reduceMotion: false,
  accent: '#1f6fe5',
};

const view = ref<PetDesktopView | null>(null);
const settings = computed(() => view.value?.settings ?? FALLBACK);
const character = computed(
  () => CHARACTERS.find((entry) => entry.id === settings.value.character) ?? CHARACTERS[0],
);
const available = computed(() =>
  (view.value?.notifications ?? []).filter((entry) =>
    settings.value.sources.includes(entry.source),
  ),
);

const row = ref(0);
const frame = ref(0);
const sheetUrl = computed(() => `url("${character.value.file}")`);
const sheetStyle = computed(() => ({
  backgroundImage: sheetUrl.value,
  backgroundSize: `1536px ${character.value.height}px`,
  height: `${character.value.height}px`,
  transform: `translate3d(${-frame.value * 192}px, ${-row.value * 208}px, 0)`,
}));

let timer: ReturnType<typeof setInterval> | undefined;
let oneShot = false;

// Rows are eight cells wide, but pets leave unused cells transparent. Playing
// them makes the pet vanish for a frame, which reads as flicker. Count the
// filled cells once per sheet with a canvas and only loop over those.
const DEFAULT_FRAMES: Record<string, number> = { idle: 7, waving: 8, jumping: 8 };
const frameCounts = ref<Record<string, number>>({ ...DEFAULT_FRAMES });
const analyzed = new Map<string, Record<string, number>>();

async function analyzeSheet(file: string) {
  const cached = analyzed.get(file);
  if (cached) {
    frameCounts.value = cached;
    return;
  }
  try {
    const image = new Image();
    image.src = file;
    await image.decode();
    const canvas = document.createElement('canvas');
    canvas.width = 1536;
    canvas.height = 208;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    if (!context) return;
    const counts: Record<string, number> = {};
    for (const [name, [rowIndex]] of Object.entries(ROWS)) {
      context.clearRect(0, 0, canvas.width, canvas.height);
      context.drawImage(image, 0, rowIndex * 208, 1536, 208, 0, 0, 1536, 208);
      const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
      let last = 0;
      for (let cell = 0; cell < 8; cell += 1) {
        let filled = false;
        for (let y = 0; y < 208 && !filled; y += 4) {
          const line = y * 1536;
          for (let x = cell * 192; x < (cell + 1) * 192; x += 4) {
            if (pixels[(line + x) * 4 + 3] > 8) {
              filled = true;
              break;
            }
          }
        }
        if (filled) last = cell + 1;
      }
      counts[name] = Math.max(1, last);
    }
    analyzed.set(file, counts);
    frameCounts.value = counts;
  } catch {
    /* Keep the defaults when the webview refuses pixel access. */
  }
}

function play(name: string, once = false) {
  clearInterval(timer);
  const [targetRow, interval] = ROWS[name] ?? ROWS.idle;
  row.value = targetRow;
  frame.value = 0;
  oneShot = once;
  if (settings.value.reduceMotion) return;
  const total = Math.max(1, frameCounts.value[name] ?? 8);
  timer = setInterval(() => {
    frame.value += 1;
    if (frame.value >= total) {
      if (oneShot) {
        oneShot = false;
        play('idle');
        return;
      }
      frame.value = 0;
    }
  }, interval);
}

const bubbleText = ref('');
const bubbleShow = ref(false);
let bubbleTimer: ReturnType<typeof setTimeout> | undefined;
function say(text?: string) {
  if (!settings.value.bubble) return;
  bubbleText.value = text ?? LINES[Math.floor(Math.random() * LINES.length)];
  bubbleShow.value = true;
  clearTimeout(bubbleTimer);
  bubbleTimer = setTimeout(() => (bubbleShow.value = false), 3200);
}

const notice = ref<PetNotification | null>(null);
const noticeOpacity = ref(1);
let noticeTimer: ReturnType<typeof setInterval> | undefined;
let noticeIndex = 0;

function rotateNotice() {
  const items = available.value;
  if (!items.length) {
    notice.value = null;
    noticeOpacity.value = 1;
    return;
  }
  const next = items[noticeIndex % items.length];
  noticeIndex += 1;
  noticeOpacity.value = 0;
  setTimeout(() => {
    notice.value = next;
    noticeOpacity.value = 1;
  }, 140);
}

function restartNotice() {
  clearInterval(noticeTimer);
  noticeIndex = 0;
  rotateNotice();
  if (available.value.length > 1) {
    noticeTimer = setInterval(rotateNotice, 6000);
  }
}

const promptOpen = ref(false);
const promptMenu = ref(false);

function showPrompt(menu: boolean) {
  bubbleShow.value = false;
  clearTimeout(bubbleTimer);
  promptOpen.value = true;
  promptMenu.value = menu;
}
function closePrompt() {
  promptOpen.value = false;
  promptMenu.value = false;
}

async function refresh() {
  try {
    const next = await bridge.petView();
    view.value = next;
    restartNotice();
    if (!timer && !settings.value.reduceMotion) play('idle');
  } catch {
    /* The window keeps its last state; the next event retries. */
  }
}

function restore() {
  void bridge.petRestore();
}

async function toggleBarless() {
  try {
    view.value = await bridge.petSetBarHidden(!settings.value.barHidden);
    restartNotice();
  } catch {
    /* Keep the menu open so the user can retry. */
  }
}

function openSettings() {
  closePrompt();
  void bridge.petOpenSettings();
}

let press: { x: number; y: number; moved: boolean; target: EventTarget | null } | null = null;
let lastPetClick = 0;

function onPointerDown(event: PointerEvent) {
  press = { x: event.clientX, y: event.clientY, moved: false, target: event.target };
}

function onPointerMove(event: PointerEvent) {
  if (!press) return;
  if (press.target instanceof Element && press.target.closest('.prompt')) return;
  if (!press.moved && Math.hypot(event.clientX - press.x, event.clientY - press.y) > 4) {
    press = null;
    // Native dragging keeps the window under the cursor without per-frame IPC.
    void getCurrentWindow().startDragging();
  }
}

function runPromptAction(action: string) {
  if (action === 'confirm') showPrompt(true);
  else if (action === 'cancel') closePrompt();
  else if (action === 'barless') void toggleBarless();
  else if (action === 'open-settings') openSettings();
}

function onPointerUp() {
  if (!press) return;
  const { moved, target } = press;
  press = null;
  if (moved) return;
  const element = target instanceof Element ? target : null;
  // A non-activating window never becomes key on macOS, so the webview may not
  // synthesize click events. Prompt actions run on pointerup instead.
  const action = element?.closest('[data-action]')?.getAttribute('data-action');
  if (action) {
    runPromptAction(action);
    return;
  }
  if (element?.closest('.prompt')) return;
  if (promptOpen.value) {
    closePrompt();
    return;
  }
  if (element?.closest('.pet')) {
    const now = Date.now();
    if (now - lastPetClick < 320) {
      lastPetClick = 0;
      showPrompt(false);
      return;
    }
    lastPetClick = now;
    play(Math.random() > 0.5 ? 'waving' : 'jumping', true);
    say();
    return;
  }
  if (element?.closest('.bar')) restore();
}

let offNotifications: (() => void) | undefined;
let offSettings: (() => void) | undefined;
let idleBubble: ReturnType<typeof setInterval> | undefined;

function pointerReset() {
  press = null;
}

watch(
  () => character.value.file,
  (file) => {
    void analyzeSheet(file);
  },
);

onMounted(async () => {
  document.documentElement.classList.add('pet-window');
  for (const entry of CHARACTERS) {
    const image = new Image();
    image.src = entry.file;
  }
  await refresh();
  await analyzeSheet(character.value.file);
  play('idle');
  offNotifications = await bridge.onPetNotifications(refresh);
  offSettings = await bridge.onPetSettings(refresh);
  idleBubble = setInterval(() => {
    if (!promptOpen.value && bubbleShow.value === false && Math.random() < 0.3) say();
  }, 24000);
  window.addEventListener('blur', pointerReset);
});

onBeforeUnmount(() => {
  document.documentElement.classList.remove('pet-window');
  clearInterval(timer);
  clearInterval(noticeTimer);
  clearInterval(idleBubble);
  clearTimeout(bubbleTimer);
  offNotifications?.();
  offSettings?.();
  window.removeEventListener('blur', pointerReset);
});
</script>

<template>
  <div
    class="pet-root"
    :class="{ barless: settings.barHidden }"
    :style="{ '--accent': settings.accent }"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
  >
    <div class="pet">
      <div class="pet-shadow" />
      <div class="pet-clip"><div class="pet-sheet" :style="sheetStyle" /></div>
      <div class="bubble" :class="{ show: bubbleShow }">{{ bubbleText }}</div>
      <div class="prompt" :class="{ open: promptOpen, menu: promptMenu }">
        <div class="prompt-ask">
          <span class="prompt-text">你想打开设置吗？</span>
          <button class="gold" data-action="confirm">确认</button>
          <button class="plain" data-action="cancel">取消</button>
        </div>
        <div class="prompt-menu">
          <div class="prompt-row" data-action="barless">
            <span class="prompt-text">只保留桌宠</span>
            <button
              class="switch"
              :class="{ off: !settings.barHidden }"
              aria-label="只保留桌宠"
              data-action="barless"
            />
          </div>
          <button class="prompt-link" data-action="open-settings">打开应用设置</button>
        </div>
      </div>
    </div>

    <div v-if="!settings.barHidden" class="bar" :class="'bar--' + settings.barStyle">
      <span class="dot" :class="{ quiet: !notice }" />
      <span class="txt" :style="{ opacity: noticeOpacity }">
        <template v-if="notice">{{ notice.text }}</template>
        <template v-else>暂无通知</template>
      </span>
      <span v-if="available.length > 1" class="count">{{ available.length }} 条</span>
    </div>
  </div>
</template>

<style>
html.pet-window,
html.pet-window body,
html.pet-window #app {
  background: transparent !important;
  overflow: hidden;
}
html.pet-window body {
  color-scheme: light;
}
</style>

<style scoped>
.pet-root {
  position: fixed;
  inset: 0;
  user-select: none;
  -webkit-user-select: none;
  color: var(--text);
  font-family: var(--font-sans);
}

/* ---------- pet ---------- */
.pet {
  position: absolute;
  left: 24px;
  bottom: 47px;
  z-index: 3;
  width: 86px;
  height: 94px;
  cursor: pointer;
}
.pet-root.barless .pet {
  bottom: 8px;
}
.pet-shadow {
  position: absolute;
  left: 18%;
  right: 18%;
  bottom: -1px;
  height: 8px;
  border-radius: 99px;
  background: radial-gradient(50% 50% at 50% 50%, rgba(15, 23, 42, 0.3), rgba(15, 23, 42, 0) 72%);
  pointer-events: none;
}
.pet-clip {
  position: absolute;
  left: 0;
  top: 0;
  width: 192px;
  height: 208px;
  overflow: hidden;
  transform: scale(0.45);
  transform-origin: top left;
}
.pet-sheet {
  position: absolute;
  left: 0;
  top: 0;
  width: 1536px;
  background-repeat: no-repeat;
  background-position: 0 0;
  will-change: transform;
}

/* ---------- bubble (app surface, no blur) ---------- */
.bubble {
  position: absolute;
  left: 50%;
  bottom: 100px;
  transform: translateX(-38%) scale(0.94);
  transform-origin: left bottom;
  max-width: 240px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 8px 12px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--text);
  box-shadow: 0 10px 24px -16px rgba(15, 23, 42, 0.4);
  white-space: nowrap;
  opacity: 0;
  pointer-events: none;
  transition:
    opacity 140ms ease,
    transform 140ms ease;
}
.bubble.show {
  opacity: 1;
  transform: translateX(-38%) scale(1);
}
.bubble::after {
  content: '';
  position: absolute;
  left: 26px;
  bottom: -5px;
  width: 9px;
  height: 9px;
  background: var(--surface);
  border-right: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
  transform: rotate(45deg);
}

/* ---------- quick settings ---------- */
.prompt {
  position: absolute;
  left: 64px;
  bottom: 4px;
  width: 232px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 14px;
  box-shadow: 0 20px 44px -26px rgba(15, 23, 42, 0.45);
  padding: 11px 13px;
  display: none;
  z-index: 5;
  cursor: default;
}
.prompt.open {
  display: block;
}
.prompt-ask {
  display: flex;
  align-items: center;
  gap: 8px;
}
.prompt-text {
  flex: 1;
  font-size: 12px;
  color: var(--text);
}
.gold {
  border: 0;
  background: none;
  color: #b8860b;
  font: inherit;
  font-size: 12.5px;
  font-weight: 650;
  cursor: pointer;
  padding: 6px 9px;
  min-height: 26px;
}
.gold:hover {
  text-decoration: underline;
}
.plain {
  border: 0;
  background: none;
  color: var(--text-tertiary, #8c929b);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
  padding: 6px 9px;
  min-height: 26px;
}
.prompt-menu {
  display: none;
}
.prompt.menu .prompt-ask {
  display: none;
}
.prompt.menu .prompt-menu {
  display: grid;
  gap: 9px;
}
.prompt-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.prompt-link {
  border: 0;
  background: var(--surface-subtle);
  color: var(--text);
  font: inherit;
  font-size: 12px;
  border-radius: 8px;
  padding: 7px 10px;
  text-align: left;
  cursor: pointer;
}
.prompt-link:hover {
  background: var(--surface-sunken);
}
.switch {
  width: 34px;
  height: 20px;
  border-radius: 99px;
  background: var(--accent);
  position: relative;
  flex: none;
  cursor: pointer;
  border: 0;
  padding: 0;
}
.switch::after {
  content: '';
  position: absolute;
  top: 2px;
  right: 2px;
  width: 16px;
  height: 16px;
  border-radius: 99px;
  background: #fff;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  transition:
    left 140ms ease,
    right 140ms ease;
}
.switch.off {
  background: #d6d9df;
}
.switch.off::after {
  right: auto;
  left: 2px;
}

/* ---------- notification bar ---------- */
.bar {
  position: absolute;
  left: 8px;
  bottom: 8px;
  z-index: 2;
  width: 320px;
  height: 40px;
  border-radius: 16px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 14px;
  cursor: pointer;
  transition:
    background 160ms ease,
    color 160ms ease;
}
.bar--glass-light {
  background: rgba(255, 255, 255, 0.5);
  border: 0;
  box-shadow:
    0 14px 32px -18px rgba(15, 23, 42, 0.45),
    inset 0 0 0 1px rgba(255, 255, 255, 0.32),
    inset 0 1px 0 rgba(255, 255, 255, 0.7);
  color: var(--text);
}
.bar--glass-dark {
  background: rgba(20, 21, 24, 0.46);
  border: 0;
  box-shadow:
    0 14px 32px -18px rgba(0, 0, 0, 0.5),
    inset 0 0 0 1px rgba(255, 255, 255, 0.14);
  color: #f4f5f7;
}
.bar--solid {
  background: rgba(255, 255, 255, 0.97);
  border: 1px solid var(--border);
  box-shadow: 0 12px 28px -18px rgba(15, 23, 42, 0.4);
  color: var(--text);
}
.dot {
  width: 7px;
  height: 7px;
  border-radius: 99px;
  background: var(--accent);
  flex: none;
}
.dot.quiet {
  background: #34a853;
}
.txt {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition: opacity 140ms ease;
}
.count {
  flex: none;
  font-size: 11px;
  border-radius: 99px;
  padding: 1px 8px;
  background: var(--accent-tint, rgba(31, 111, 229, 0.13));
  color: var(--accent-text, #1a5ec6);
}
.bar--glass-dark .count {
  background: rgba(130, 172, 255, 0.22);
  color: #d4e2ff;
}

@media (prefers-reduced-motion: reduce) {
  .bubble,
  .txt,
  .switch::after {
    transition: none;
  }
}
</style>
