<script setup lang="ts">
import type { Component } from 'vue';
import {
  ArrowRight,
  Bot,
  Camera,
  Cat,
  Check,
  Download,
  FolderOpen,
  Globe,
  HardDrive,
  HardDriveUpload,
  Home,
  ImagePlus,
  LockKeyhole,
  Package,
  PanelLeft,
  Pencil,
  Pipette,
  Plug,
  Plus,
  RefreshCw,
  Server,
  SlidersHorizontal,
  Trash2,
} from 'lucide-vue-next';
import {
  PhArrowsClockwise,
  PhArrowRight,
  PhCamera,
  PhCat,
  PhCheck,
  PhDatabase,
  PhDownloadSimple,
  PhEyedropper,
  PhFolderOpen,
  PhGlobe,
  PhHardDrives,
  PhHouse,
  PhImageSquare,
  PhLockKey,
  PhPackage,
  PhPencilSimple,
  PhPlugs,
  PhPlus,
  PhRobot,
  PhSidebarSimple,
  PhSlidersHorizontal,
  PhTrash,
  PhUploadSimple,
} from '@phosphor-icons/vue';
import {
  IconAdjustmentsHorizontal,
  IconArrowRight,
  IconCamera,
  IconCat,
  IconCheck,
  IconColorPicker,
  IconDatabase,
  IconDownload,
  IconFolderOpen,
  IconHome,
  IconLayoutSidebar,
  IconLock,
  IconPackage,
  IconPencil,
  IconPhotoPlus,
  IconPlugConnected,
  IconPlus,
  IconRefresh,
  IconRobot,
  IconServer,
  IconTrash,
  IconUpload,
  IconWorld,
} from '@tabler/icons-vue';

/**
 * 开发用图标对比页（/src/dev-icons.html），只为挑选图标库，不是产品页面。
 * 三种库都按 1:1 的实际像素渲染，尺寸和图标底框与应用设置保持一致。
 */

type SetKey = 'lucide' | 'phosphor' | 'tabler';
interface Row {
  label: string;
  lucide: Component;
  phosphor: Component;
  tabler: Component;
}

const sets: { key: SetKey; name: string }[] = [
  { key: 'lucide', name: 'Lucide（当前）' },
  { key: 'phosphor', name: 'Phosphor' },
  { key: 'tabler', name: 'Tabler' },
];

const settingsTabs = [
  { label: '外观与交互', active: true },
  { label: '主页' },
  { label: '桌面宠物' },
  { label: '插件服务' },
  { label: '软件更新' },
  { label: '存储' },
];

const rows: Row[] = [
  {
    label: '外观与交互',
    lucide: SlidersHorizontal,
    phosphor: PhSlidersHorizontal,
    tabler: IconAdjustmentsHorizontal,
  },
  { label: '主页', lucide: Home, phosphor: PhHouse, tabler: IconHome },
  { label: '桌面宠物', lucide: Cat, phosphor: PhCat, tabler: IconCat },
  { label: '插件服务', lucide: Package, phosphor: PhPackage, tabler: IconPackage },
  { label: '软件更新', lucide: Download, phosphor: PhDownloadSimple, tabler: IconDownload },
  { label: '存储', lucide: HardDrive, phosphor: PhHardDrives, tabler: IconServer },
  { label: '文件夹', lucide: FolderOpen, phosphor: PhFolderOpen, tabler: IconFolderOpen },
  { label: '迁移上传', lucide: HardDriveUpload, phosphor: PhUploadSimple, tabler: IconUpload },
  { label: '只读锁定', lucide: LockKeyhole, phosphor: PhLockKey, tabler: IconLock },
  { label: '前进箭头', lucide: ArrowRight, phosphor: PhArrowRight, tabler: IconArrowRight },
  { label: '完成勾选', lucide: Check, phosphor: PhCheck, tabler: IconCheck },
  { label: '取色', lucide: Pipette, phosphor: PhEyedropper, tabler: IconColorPicker },
  { label: '头像相机', lucide: Camera, phosphor: PhCamera, tabler: IconCamera },
  { label: '添加图片', lucide: ImagePlus, phosphor: PhImageSquare, tabler: IconPhotoPlus },
  { label: 'Agent', lucide: Bot, phosphor: PhRobot, tabler: IconRobot },
  { label: '侧边栏', lucide: PanelLeft, phosphor: PhSidebarSimple, tabler: IconLayoutSidebar },
  { label: 'MCP 插件', lucide: Plug, phosphor: PhPlugs, tabler: IconPlugConnected },
  { label: '服务器', lucide: Server, phosphor: PhDatabase, tabler: IconServer },
  { label: '刷新', lucide: RefreshCw, phosphor: PhArrowsClockwise, tabler: IconRefresh },
  { label: '新增', lucide: Plus, phosphor: PhPlus, tabler: IconPlus },
  { label: '删除', lucide: Trash2, phosphor: PhTrash, tabler: IconTrash },
  { label: '编辑', lucide: Pencil, phosphor: PhPencilSimple, tabler: IconPencil },
  { label: 'Web', lucide: Globe, phosphor: PhGlobe, tabler: IconWorld },
];

const tabIcons: Record<SetKey, Component[]> = {
  lucide: [SlidersHorizontal, Home, Cat, Package, Download, HardDrive],
  phosphor: [PhSlidersHorizontal, PhHouse, PhCat, PhPackage, PhDownloadSimple, PhHardDrives],
  tabler: [IconAdjustmentsHorizontal, IconHome, IconCat, IconPackage, IconDownload, IconServer],
};

const rowIcons: Record<SetKey, [Component, Component, Component]> = {
  lucide: [PanelLeft, Plug, Server],
  phosphor: [PhSidebarSimple, PhPlugs, PhDatabase],
  tabler: [IconLayoutSidebar, IconPlugConnected, IconServer],
};

const sampleRows = [
  { title: '在侧边栏显示 Agent', note: '关闭后仍可从右上角进入' },
  { title: 'MCP 模式', note: '本机接口，供外部工具调用' },
  { title: '模型服务', note: 'OpenAI 兼容 / Anthropic / Gemini' },
];

const weights = ['thin', 'light', 'regular', 'bold', 'fill', 'duotone'] as const;
const weightRows = [
  { label: '外观与交互', icon: PhSlidersHorizontal },
  { label: '主页', icon: PhHouse },
  { label: '存储', icon: PhHardDrives },
  { label: 'Agent', icon: PhRobot },
];
</script>
<template>
  <main class="sheet">
    <header class="sheet-head">
      <h1>应用设置图标对比</h1>
      <p>三种图标库在同一尺寸下渲染；图标底框与应用设置的行首图标一致。</p>
    </header>

    <section>
      <h2>设置分类</h2>
      <div class="stack">
        <article v-for="set in sets" :key="set.key" class="mock">
          <h3>{{ set.name }}</h3>
          <div class="tab-row">
            <span
              v-for="(tab, index) in settingsTabs"
              :key="tab.label"
              class="tab"
              :class="{ active: tab.active }"
            >
              <component :is="tabIcons[set.key][index]" :size="14" />
              {{ tab.label }}
            </span>
          </div>
        </article>
      </div>
    </section>

    <section>
      <h2>设置行首图标</h2>
      <div class="columns">
        <article v-for="set in sets" :key="set.key" class="mock">
          <h3>{{ set.name }}</h3>
          <div class="setting-card">
            <div v-for="(row, index) in sampleRows" :key="row.title" class="setting-row">
              <span class="tile">
                <component :is="rowIcons[set.key][index]" :size="16" />
              </span>
              <span class="copy">
                <strong>{{ row.title }}</strong>
                <small>{{ row.note }}</small>
              </span>
            </div>
          </div>
        </article>
      </div>
    </section>

    <section>
      <h2>同尺寸对照</h2>
      <table class="grid">
        <thead>
          <tr>
            <th>用途</th>
            <th>Lucide（当前）</th>
            <th>Phosphor</th>
            <th>Tabler</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in rows" :key="row.label">
            <th scope="row">{{ row.label }}</th>
            <td v-for="set in sets" :key="set.key">
              <span class="sizes">
                <component :is="row[set.key]" :size="16" />
                <component :is="row[set.key]" :size="18" />
                <component :is="row[set.key]" :size="24" />
              </span>
            </td>
          </tr>
        </tbody>
      </table>
    </section>

    <section>
      <h2>Phosphor 粗细</h2>
      <div class="weights">
        <div v-for="weight in weights" :key="weight" class="weight">
          <p>{{ weight }}</p>
          <span v-for="row in weightRows" :key="row.label" class="weight-icons">
            <component :is="row.icon" :size="18" :weight="weight" />
          </span>
        </div>
      </div>
    </section>
  </main>
</template>
<style scoped>
.sheet {
  max-width: 980px;
  margin: 0 auto;
  padding: 36px 32px 64px;
  color: var(--text);
  background: var(--surface);
  font-family: var(--font-sans);
}
.sheet-head h1 {
  margin: 0;
  font-size: 22px;
  font-weight: 600;
}
.sheet-head p {
  margin: 6px 0 0;
  color: var(--text-tertiary);
  font-size: 12.5px;
}
section {
  margin-top: 34px;
}
section > h2 {
  margin: 0 0 12px;
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 600;
}
.mock h3 {
  margin: 0 0 6px;
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-weight: 500;
}
.stack {
  display: grid;
  gap: 18px;
}
.columns {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 14px;
}
.tab-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--surface);
}
.tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 12.5px;
}
.tab.active {
  background: var(--accent-soft);
  color: var(--accent);
}
.setting-card {
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--surface);
  overflow: hidden;
}
.setting-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
}
.setting-row + .setting-row {
  border-top: 1px solid var(--divider);
}
.tile {
  display: grid;
  place-items: center;
  flex-shrink: 0;
  width: 30px;
  height: 30px;
  border-radius: 9px;
  background: var(--accent-soft);
  color: var(--accent);
}
.copy {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.copy strong {
  font-size: 12.5px;
  font-weight: 500;
}
.copy small {
  margin-top: 1px;
  color: var(--text-tertiary);
  font-size: 11px;
}
.grid {
  width: 100%;
  border-collapse: collapse;
}
.grid th,
.grid td {
  padding: 7px 10px;
  border-bottom: 1px solid var(--divider);
  text-align: left;
  font-size: 12.5px;
  font-weight: 400;
}
.grid thead th {
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-weight: 500;
}
.grid tbody th {
  width: 110px;
  color: var(--text-secondary);
}
.sizes {
  display: inline-flex;
  align-items: center;
  gap: 14px;
}
.weights {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
.weight {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
}
.weight p {
  width: 54px;
  margin: 0;
  color: var(--text-tertiary);
  font-size: 11px;
}
.weight-icons {
  display: grid;
  place-items: center;
  width: 26px;
  color: var(--text-secondary);
}
</style>
