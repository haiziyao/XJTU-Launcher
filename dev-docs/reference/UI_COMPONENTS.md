# 按钮组件与用法

更新于 2026-10-31。所有页面按钮统一使用 `src/components/ui/` 下的两个组件；页面里不再直接写 `class="btn ..."`，也不再为单个页面另造图标按钮。

## UiButton

文字操作按钮，覆盖原来的 `.btn` 类。样式仍定义在 `src/styles/ui.css`（`.btn` 及其变体），组件只负责组合类名，因此页面级 scoped 规则（如 `.saved-password .btn`）继续生效。

```vue
<UiButton @click="save">保存</UiButton>
<UiButton variant="primary" :disabled="busy" @click="submit">提交</UiButton>
<UiButton variant="ghost" size="sm" @click="cancel">取消</UiButton>
<UiButton variant="ghost" size="sm" danger @click="remove">删除</UiButton>
```

| 属性      | 取值        | 默认    | 用途                                                |
| --------- | ----------- | ------- | --------------------------------------------------- |
| `variant` | `secondary` | ✓       | 白底描边，页面里的普通操作                          |
|           | `primary`   |         | 每处流程只放一个主操作：保存、提交、下载更新        |
|           | `tinted`    |         | 浅蓝底的轻量操作，如侧栏“新对话”“工作区”            |
|           | `ghost`     |         | 无边框，用于标题旁、行内、表单取消                  |
| `size`    | `md`        | ✓       | 高 30px，用于表单和页面主操作                       |
|           | `sm`        |         | 高 26px，用于行尾、分组标题和工具条                 |
| `danger`  | 布尔        | `false` | 只改文字颜色；与 `ghost` 搭配是破坏性操作的常见写法 |

`type`、`disabled`、`aria-*`、`@click` 等原生属性直接透传到 `<button>`；组件不设置默认 `type`，表单内的提交行为与改动前一致。图标放在默认插槽里，尺寸用 13–14。

## UiIconButton

只有图标的方形或圆形按钮。`label` 必填，作为 `aria-label`；禁用态沿用全局 `button:disabled`。

```vue
<UiIconButton label="刷新课表" @click="reload"><RefreshCw :size="14" /></UiIconButton>
<UiIconButton
  variant="accent"
  size="md"
  label="新对话"
  @click="newChat"
><Plus :size="15" /></UiIconButton>
<UiIconButton
  variant="primary"
  size="md"
  shape="circle"
  type="submit"
  label="发送"
  :disabled="!canSend"
>
  <ArrowUp :size="15" />
</UiIconButton>
<UiIconButton variant="danger" label="删除记忆" @click="remove"><Trash2 :size="14" /></UiIconButton>
```

| 属性      | 取值      | 默认    | 用途                                                          |
| --------- | --------- | ------- | ------------------------------------------------------------- |
| `label`   | 字符串    | 必填    | 无障碍名称；同时作为 hover 提示之外的读屏文本                 |
| `variant` | `quiet`   | ✓       | 行尾、字段内的操作；hover 为淡灰底                            |
|           | `accent`  |         | Agent 工具条等强调型图标；hover 为淡蓝底和蓝字                |
|           | `primary` |         | 实心主按钮，如发送（配合 `circle`）                           |
|           | `neutral` |         | 实心但低调的状态，如“停止生成”                                |
|           | `danger`  |         | 删除类操作；hover 为淡红底红字                                |
|           | `inherit` |         | 跟随容器颜色，用于提示条上的关闭按钮                          |
| `size`    | `xs`      |         | 20px，行内小关闭按钮                                          |
|           | `sm`      | ✓       | 24px，行尾和字段操作的默认值                                  |
|           | `md`      |         | 28px，工具栏                                                  |
| `shape`   | `square`  | ✓       | 圆角 6px                                                      |
|           | `circle`  |         | 正圆，用于发送 / 停止                                         |
| `active`  | 布尔      | `false` | 当前选中或开启状态的底色；需要读屏状态时另行传 `aria-pressed` |

按钮自身的样式写在组件内；页面只通过外层容器控制间距，不再覆盖图标按钮的尺寸和 hover。需要“默认厂商”这类一次性着色时，可以给组件加类（例如 ProvidersView 的 `class="ui-icon-button accent"` 对应的 `.ui-icon-button.accent`）。

## PageTabs 与 SegmentedTabs

两种标签栏按层级分工，不要互相替代，也不要在页面里重写其中任何一种：

- `PageTabs`：页面级切换，紧跟 `PageHeader`，用于“应用设置”“Agent 设置”“插件中心”这类整页分区。透明底、底部一条细线，选中项用主题色浅底和主题色文字，不再出现灰色胶囊底。
- `SegmentedTabs`：页面内部的小范围筛选，位于某个分组上方或内容区里，使用时传 `compact`（例如思源学堂课程内的活动分类，带数量、条目多可横向滚动）。它保留浅灰底容器和白底选中胶囊。

```vue
<PageTabs v-model="tab" :tabs="sections" label="设置分类" />
<SegmentedTabs v-model="active" class="lms-tabs" compact :tabs="tabs" label="活动分类" />
```

两者都接收 `{ id, title }[]`、`v-model` 和必填的 `label`（`aria-label`），并支持方向键 / Home / End 切换。页面不再用 `:deep()` 覆盖标签栏颜色、底色或间距。选中态保持实色浅底（`--accent-soft` / 白底），不要换成半透明色，否则壁纸纹理会穿过文字。

## 控件底色与透明度

有壁纸时，页面和卡片是半透明的（`--panel`），控件不能再用纯白盖住壁纸，也不能各写一套颜色。统一使用下面的 token（定义在 `style.css`，`desktop-shell.css` 的 `.desktop-shell.has-wallpaper` 负责切换）：

| token                 | 无壁纸    | 有壁纸                  | 用途                           |
| --------------------- | --------- | ----------------------- | ------------------------------ |
| `--control-bg`        | `#fff`    | `rgba(255,255,255,.55)` | 按钮、输入框、选择器、周次选择 |
| `--control-bg-subtle` | `#f7f8fa` | `rgba(255,255,255,.4)`  | 页头搜索框等次要输入           |
| `--control-border`    | `#d4d7dc` | `rgba(15,23,42,.1)`     | 控件描边；壁纸下仍要能看出边界 |
| `--control-sunken`    | `#f1f2f4` | `rgba(15,23,42,.05)`    | 分段标签的轨道                 |

- 大面板继续用 `--panel` / `--panel-border`；控件只改底色和描边，不改尺寸。
- 原生 `<select>` 一律 `appearance: none`，箭头用 `--chevron-down`（12px 灰色 chevron，与 lucide 图标一致），不要再让 macOS 画出白色胶囊。
- 弹层（下拉菜单、对话框）可以保持实色，保证可读性。

## 不属于按钮组件的控件

以下控件保留各自的语义和样式，不要套用上面的组件，也不要在页面里另写一套“看起来像按钮”的样式：

- 导航行：`desktop-nav-item`、`history-open`、`router` 条目、`plugin-tile-main`、`agent-tool-row`、`campus-record`（整行可点或只是数据行）。
- 选择卡片：`pet-option`、`accent-swatch`、`clock-option`、头像 / 壁纸选择。
- 开关：`UiSwitch`。
- 选择器触发器：`MenuSelect` 的 `menu-field-value`。
- 布局控件：侧栏折叠 `sidebar-toggle`、窗口按钮、`profile-avatar`。
- 通知条上的关闭按钮使用 `UiIconButton variant="inherit" size="xs"`，只保留 `notice-close` 作为外边距钩子。

新增页面时先在这几个组件和控件里选，确实没有合适语义再提新组件，并在本文件补充用法。

## v0.6 校园组件套件（2026-10-31）

按 [LAYOUT §4.2](../../docs/design/campus-v0.6/LAYOUT.md) 实现的组件，放在 `src/components/ui/`（通用）和 `src/features/campus/components/`（校园专用）。

**当前状态**：2026-10-31 经用户明确授权“直接全量接入”，组件已接进 `CampusView` 的校园工作区（LAYOUT §2.1 / §3）。评教分区视觉仍未获准，`CampusEvaluations.vue` 外观未改。开发画廊仍保留：`npm run tauri dev` 后点侧栏「组件」；画廊由 `import.meta.env.DEV` 保护，生产包不导入、也不显示入口，其中的数据全部是示例。

### 分区、标题与骨架

| 组件                        | 用途                         | 关键 props / 事件                                                                                        |
| --------------------------- | ---------------------------- | -------------------------------------------------------------------------------------------------------- |
| `ui/SectionHeader`          | 分区标题一行：标题、更新时间 | `title`、`subtitle`、`updatedAt`、`refreshing`、`sticky`；插槽 `filters` / `actions` / `more`；`refresh` |
| `campus/CampusRail`         | 校园分区侧栏                 | `v-model`（分区 id）、`items: RailItem[]`、`collapsed`；`select`。禁用项必须带 `reason`，保持可见        |
| `ui/SplitView`              | 列表 + 详情分栏              | `v-model:width`、`detailOpen`、`listLabel` / `detailLabel`；插槽 `list` / `detail`；`back`               |
| `ui/StatTile`、`ui/StatRow` | 指标块与指标行               | `label`、`value`、`note`、`loading`、`failed`、`clickable`、`size`；`select` / `retry`                   |
| `ui/Skeleton`               | 首屏占位                     | `width`、`height`、`radius`、`lines`、`label`                                                            |
| `ui/FilterBar`              | 搜索 + 页面自带筛选控件      | `v-model:query`、`active`、`placeholder`、`label`；插槽默认（筛选控件）与 `actions`；`clear`             |

`SplitView` 按自身宽度切换布局：≥1000 双栏且分隔条可拖（键盘方向键 / Home / End），760–999 列表固定 280px，更窄时单栏并由 `back` 返回列表。它用 ResizeObserver 测量内容宽度，不要给外层再加固定宽度假设。

`CampusRail` 的滑动指示器要求滚动的 `<nav>` 是按钮的 offsetParent；分组容器不得加 `position: relative`。

### 列表、表格与文件

| 组件              | 用途                | 关键 props / 事件                                                                                                                           |
| ----------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `ui/ListItem`     | 作业 / 通知列表行   | `title`、`subtitle`、`selected`、`disabled`、`unread`；插槽 `leading` / `meta` / `status` / `subtitle`；`select`                            |
| `ui/DataTable`    | 成绩、考试等表格    | `columns`、`rows`、`rowKey`、`v-model:sort`、`v-model:selected`、`selectable`、`loading`、`stickyHeader`；插槽 `cell-<key>`；`select`       |
| `campus/FileRow`  | 课程 / 通知附件一行 | `name`、`size`、`kind`、`downloaded`、`detail`、`previewable`、`downloadable`、`busy`；`preview` / `download`；批量模式的 `v-model:checked` |
| `campus/DropZone` | 作业附件拖放区      | `maxFiles`（10）、`maxFileSize`、`maxTotal`、`accept`、`disabled`、`hint`；`files` / `reject`                                               |

`DataTable` 的排序与选择由页面持有，组件只发 `update:*`；数字列传 `align: 'right'`，自动使用等宽数字。`DropZone` 依赖 Tauri 窗口的 `dragDropEnabled: false`，否则系统事件会盖住 HTML5 拖放。

### 状态、期限与提示

| 组件             | 用途                   | 关键 props / 事件                                                                              |
| ---------------- | ---------------------- | ---------------------------------------------------------------------------------------------- |
| `ui/StatusBadge` | 作业 / 考试 / 预约状态 | `status: done \| near \| urgent \| active \| neutral \| new`、`label`                          |
| `ui/DueLabel`    | 截止时间 + 倒计时      | `at`、`now`、`done`、`countdown`；颜色和图标来自 `dueInfo`                                     |
| `ui/InlineAlert` | 页面内提示条           | `tone: info \| warning \| error \| partial`、`title`、`dismissible`；插槽 `actions`；`dismiss` |
| `ui/MoreMenu`    | 「⋯」次要操作菜单      | `items: MoreItem[]`、`align`、`label`；`select(id)`                                            |

状态永远同时有图标和文字（`new` 是 6px 圆点加读屏文本），不能只靠颜色。`dueInfo` 的红色阈值是已过期或 ≤3 小时，黄色 ≤24 小时；超过 30 天不显示倒计时。

### 详情、预览与写操作

| 组件                       | 用途                       | 关键 props / 事件                                                                                                                                                |
| -------------------------- | -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `campus/ReaderView`        | 通知正文阅读               | `html`、`title`、`source`、`date`、`attachments`、`truncated`、`loading`、`error`；`openLink` / `download`                                                       |
| `campus/FilePreviewDialog` | 图片 / 文本 / PDF 预览弹窗 | `open`、`preview: FilePreview`、`loading`、`error`、`pdfSource`；`close` / `download`                                                                            |
| `campus/WritePreviewPanel` | 所有学校写操作的预览与确认 | `preview: WritePreview`、`busy`、`result: WriteOutcome`、`context`、`cancelLabel`、`confirmLabel`、`now`；`cancel` / `confirm(captcha)` / `viewRecords` / `done` |
| `campus/SlotMatrix`        | 场馆时间 × 场地网格        | `times`、`courts`、`cells`、`v-model`（stock id 列表）、`selectable`；插槽 `bar`；`select`                                                                       |
| `campus/SeatMap`           | 图书馆座位图               | `seats: LibrarySeat[]`、`selected`、`selectable`；`select(seat)`                                                                                                 |

`ReaderView` 会再清洗一次后端 HTML（禁止 `style`、表单、iframe 等），链接不导航应用窗口，而是发 `openLink` 交由页面打开校园网页。

`WritePreviewPanel` **绝不自行提交**：没有预览就不渲染确认按钮，预览过期后确认按钮禁用，`unknown` 结果只提供「查看提交记录」且不重试。接入时必须保持“预览 → 用户明确确认 → `confirm=true`”的顺序，取消不能说成撤回。

`FilePreviewDialog` 用 `pdfjs-dist` 的 legacy 构建，PDF 字节由调用方通过 `pdfSource` 提供（后端只给预览 id）。接入时还要：

- 把 `pdfjs-dist` 的 `cmaps/`、`standard_fonts/`、`wasm/` 复制到 `public/` 并设置 `cMapUrl` / `standardFontDataUrl` / `wasmUrl`，否则中文 PDF 会缺字；
- 在 Tauri CSP 增加 `worker-src 'self'`，评教视频另加 `media-src https://review-class.xjtu.edu.cn`。

### 图表

`ui/MiniBars`（`values: { label, value, highlight? }[]`）用于 7 天支出这类短序列，最多 24px 宽、2px 间隔、4px 圆角数据端；`ui/Sparkline`（`points: { label, value }[]`）用于绩点趋势，2px 折线、≤12 点时显示 4px 标记。两者都是手写 SVG、单一坐标轴、文字只用文字 token，并提供 hover / 键盘提示和「查看数据」表格。`Sparkline` 少于 2 个点显示“数据不足”，不画空图。

### 纯函数与 token

`src/shared/domain.ts` 新增（均有单元测试）：`campusTimestamp` / `campusParts`（固定 UTC+8 校区时间）、`dueInfo`、`updatedInfo`、`money`、`fileSize`、`percent`、`gradeStats`、`todoBuckets`、`parseExamTime`、`sectionTime`、`attendanceStats`。

`src/styles/style.css` 新增三个 token：`--success-text: #246e44`（浅绿底上的小字）、`--warning: #a15c00`、`--warning-soft: #fdf3e6`。成功与警告色在色觉缺陷模拟下 ΔE 只有 7.2，所以状态必须同时有图标、文字和 ≥2px 间距。

## 检查

前端改动后运行 `npm run build`、`npm test`、`npm run format:check`。用 `grep -rn '<button' src/components` 确认没有遗漏的裸 `.btn` 类；`cargo` 相关检查仅在后端同时改动时需要。
