---
name: xjtul-style
description: 修改 XJTU Launcher 的前端视觉、样式、页面布局或通用组件时使用。包含设计审批前置条件、设计 token、UiButton/PageTabs 等组件选择、壁纸透明度、排版与验收清单。
---

# XJTU Launcher 样式与界面

## 先确认能不能改

1. **新页面 / 新视觉必须先有参考图，且用户明确同意设计与开始实现。**
   - “先做个页面”“技术方案可以”“出张图看看”都不等于设计获批。
   - 已获准的页面可以继续调整细节；未获准的页面只借鉴骨架，不改视觉。
2. 已知未获准的区域：
   - `docs/design/campus-agent-v1/` 的 Agent / 评教页。
   - 最小化桌宠的设计。
   - `docs/design/style-v1/` 只是提案，未改代码。
   - 畅行校园 B 版只获准了导航、待办、课程资料版式，配色与整体样式仍在返工。
3. 如果任务只是修 bug、跟随现有样式，不需要重新审批；但仍要遵守下面的规则。

## 规则来源

- 完整规则：[dev-docs/CONVENTIONS.md](../../dev-docs/CONVENTIONS.md) 第 2 节。
- 组件用法：[dev-docs/reference/UI_COMPONENTS.md](../../dev-docs/reference/UI_COMPONENTS.md)。
- 设计稿与审批状态：[docs/README.md](../../docs/README.md)。

## 样式文件分工

| 文件                      | 内容                                   |
| ------------------------- | -------------------------------------- |
| `src/styles/style.css`           | 设计 token、全局重置、字体栈           |
| `src/styles/ui.css`              | 共用控件：`.btn`、输入框、开关、标签等 |
| `src/styles/pages.css`           | 页面级布局与组件样式                   |
| `src/styles/desktop-shell.css`   | 侧栏、窗口外壳、壁纸模式               |
| `src/styles/campus.css`          | 畅行校园专用                           |
| `src/components/ui/*.vue` | 通用组件，优先复用                     |

不要新建一套平行的颜色、圆角、阴影或动效变量。

## Token 速查

核心 token 在 `src/styles/style.css` 的 `:root`：

- 文字：`--text` / `--text-secondary` / `--text-tertiary`。
- 表面：`--surface` / `--surface-subtle` / `--surface-sunken` / `--surface-hover` / `--surface-pressed`。
- 边框：`--border` / `--border-strong` / `--divider`。
- 控件：`--control-bg` / `--control-bg-subtle` / `--control-border` / `--control-sunken`。
- 强调：`--accent` / `--accent-hover` / `--accent-pressed` / `--accent-soft` / `--accent-text` / `--accent-tint`。
- 状态：`--danger`、`--success`、`--success-text`、`--warning` 及各自的 `-soft`。
- 圆角：`--radius-xs` / `--radius-sm` / `--radius` / `--radius-lg` / `--radius-full`。
- 动效：`--ease-out` / `--ease-in` / `--ease-standard`、`--duration-fast` / `--duration` / `--duration-slow`。

有壁纸时，`.desktop-shell.has-wallpaper` 会把 `--panel`、`--control-bg` 等切成半透明。**不要写死白色或 `#fff` 当作控件底色**，否则壁纸下会像贴片。弹层可以保持实色以保证可读性。

## 组件选择

| 场景             | 用什么                              |
| ---------------- | ----------------------------------- |
| 文字按钮         | `UiButton`，不要写 `class="btn"`    |
| 纯图标按钮       | `UiIconButton`，必须传 `label`      |
| 页面级分区切换   | `PageTabs`                          |
| 分组内的紧凑筛选 | `SegmentedTabs` + `compact`         |
| 设置行 / 分组    | `SettingRow` / `SettingGroup`       |
| 列表行           | `ListItem`                          |
| 表格             | `DataTable`                         |
| 状态             | `StatusBadge`，必须同时有图标或文字 |
| 截止时间         | `DueLabel`                          |
| 页面提示         | `InlineAlert`                       |
| 次要操作         | `MoreMenu`                          |
| 空状态           | `EmptyState`                        |

原生 `<select>` 必须 `appearance: none`，箭头用 `--chevron-down`；需要下拉选择时优先 `MenuSelect`。导航行、选择卡片、开关、侧栏折叠等保留各自语义，不要硬套按钮组件。

## 排版

- 页面标题、分区标题、卡片标题、标签、正文、附注各用现有层级，不要每个页面手写一套字号。
- 中文不要用负字距；`-0.01em` 是拉丁字母技巧，对中文无效。
- 正文默认 13px / 1.6；设置页信息密度要克制。
- 不添加无关页脚、学校名、版本号、数量统计、状态灯、宣传卡片或占位演示对话。
- 默认以文字呈现资料，用户操作后再进入编辑态。

## 设置页

- 用列表 + 弹出卡片，不写解释性小字；确实需要说明时先征得用户同意。
- 新增设置项放在列表末尾。
- 破坏性操作要确认；路径显示为 `~/...`，不能出现用户名。

## 壁纸与桌面外壳

- 壁纸必须能在整个界面（包括顶部）看到；不要用整片蒙版或底板把壁纸盖住。
- 页面和卡片用 `--panel` / `--panel-border`；控件用 `--control-bg` / `--control-border`。
- 内容区右上角的“清晰 / 均衡 / 透亮”按钮只在有壁纸时出现，三档共用同一份配置。
- 侧栏收起宽度：macOS 76px（容纳窗口按钮），其他平台 56px；侧栏开关位于分隔线垂直中部。
- 最小化桌宠只拥有自己的少量命令，不获得主窗口 IPC；关闭窗口仍然是关闭。

## 交互与无障碍

- 所有交互元素都要有 `:focus-visible` 样式；图标按钮必须有 `aria-label`。
- 状态不能只靠颜色；成功 / 警告在色觉缺陷下可能接近，必须同时有图标、文字或间距。
- 支持键盘操作（方向键 / Home / End / Esc），下拉与弹层要能点外关闭。
- 尊重 `prefers-reduced-motion`；动画不能阻塞操作，也不能在收起过程中闪现内容。

## 常见反模式

- 页面里直接写 `<button class="btn">` 或另造 `.icon-button`。
- 用纯白 / 纯黑硬编码盖住壁纸。
- 用 `:deep()` 覆盖 `PageTabs` / `SegmentedTabs` 的颜色和间距。
- 把 `SegmentedTabs` 当页面级标签，或把 `PageTabs` 塞进分组。
- 给按钮加不属于 token 的阴影、渐变、发光。
- 用颜色单独表达“已交 / 未交 / 错误”。

## 完成后检查

```sh
npm run build
npm test
npm run format:check
grep -rn '<button' src/components   # 确认没有遗漏的裸按钮
```

较大改动再跑 Rust 检查。最后说明：改了哪些页面、是否真机看过、哪些状态还没验收。
