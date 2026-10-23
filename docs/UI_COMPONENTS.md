# 按钮组件与用法

更新于 2026-10-23。所有页面按钮统一使用 `src/components/ui/` 下的两个组件；页面里不再直接写 `class="btn ..."`，也不再为单个页面另造图标按钮。

## UiButton

文字操作按钮，覆盖原来的 `.btn` 类。样式仍定义在 `src/ui.css`（`.btn` 及其变体），组件只负责组合类名，因此页面级 scoped 规则（如 `.saved-password .btn`）继续生效。

```vue
<UiButton @click="save">保存</UiButton>
<UiButton variant="primary" :disabled="busy" @click="submit">提交</UiButton>
<UiButton variant="ghost" size="sm" @click="cancel">取消</UiButton>
<UiButton variant="ghost" size="sm" danger @click="remove">删除</UiButton>
```

| 属性 | 取值 | 默认 | 用途 |
| --- | --- | --- | --- |
| `variant` | `secondary` | ✓ | 白底描边，页面里的普通操作 |
| | `primary` | | 每处流程只放一个主操作：保存、提交、下载更新 |
| | `tinted` | | 浅蓝底的轻量操作，如侧栏“新对话”“工作区” |
| | `ghost` | | 无边框，用于标题旁、行内、表单取消 |
| `size` | `md` | ✓ | 高 30px，用于表单和页面主操作 |
| | `sm` | | 高 26px，用于行尾、分组标题和工具条 |
| `danger` | 布尔 | `false` | 只改文字颜色；与 `ghost` 搭配是破坏性操作的常见写法 |

`type`、`disabled`、`aria-*`、`@click` 等原生属性直接透传到 `<button>`；组件不设置默认 `type`，表单内的提交行为与改动前一致。图标放在默认插槽里，尺寸用 13–14。

## UiIconButton

只有图标的方形或圆形按钮。`label` 必填，作为 `aria-label`；禁用态沿用全局 `button:disabled`。

```vue
<UiIconButton label="刷新课表" @click="reload"><RefreshCw :size="14" /></UiIconButton>
<UiIconButton variant="accent" size="md" label="新对话" @click="newChat"><Plus :size="15" /></UiIconButton>
<UiIconButton variant="primary" size="md" shape="circle" type="submit" label="发送" :disabled="!canSend">
  <ArrowUp :size="15" />
</UiIconButton>
<UiIconButton variant="danger" label="删除记忆" @click="remove"><Trash2 :size="14" /></UiIconButton>
```

| 属性 | 取值 | 默认 | 用途 |
| --- | --- | --- | --- |
| `label` | 字符串 | 必填 | 无障碍名称；同时作为 hover 提示之外的读屏文本 |
| `variant` | `quiet` | ✓ | 行尾、字段内的操作；hover 为淡灰底 |
| | `accent` | | Agent 工具条等强调型图标；hover 为淡蓝底和蓝字 |
| | `primary` | | 实心主按钮，如发送（配合 `circle`） |
| | `neutral` | | 实心但低调的状态，如“停止生成” |
| | `danger` | | 删除类操作；hover 为淡红底红字 |
| | `inherit` | | 跟随容器颜色，用于提示条上的关闭按钮 |
| `size` | `xs` | | 20px，行内小关闭按钮 |
| | `sm` | ✓ | 24px，行尾和字段操作的默认值 |
| | `md` | | 28px，工具栏 |
| `shape` | `square` | ✓ | 圆角 6px |
| | `circle` | | 正圆，用于发送 / 停止 |
| `active` | 布尔 | `false` | 当前选中或开启状态的底色；需要读屏状态时另行传 `aria-pressed` |

按钮自身的样式写在组件内；页面只通过外层容器控制间距，不再覆盖图标按钮的尺寸和 hover。需要“默认厂商”这类一次性着色时，可以给组件加类（例如 ProvidersView 的 `class="ui-icon-button accent"` 对应的 `.ui-icon-button.accent`）。

## PageTabs 与 SegmentedTabs

两种标签栏按层级分工，不要互相替代，也不要在页面里重写其中任何一种：

- `PageTabs`：页面级切换，紧跟 `PageHeader`，用于“应用设置”“Agent 设置”“插件市场”这类整页分区。透明底、底部一条细线，选中项用主题色浅底和主题色文字，不再出现灰色胶囊底。
- `SegmentedTabs`：页面内部的小范围筛选，位于某个分组上方或内容区里，使用时传 `compact`（例如思源学堂课程内的活动分类，带数量、条目多可横向滚动）。它保留浅灰底容器和白底选中胶囊。

```vue
<PageTabs v-model="tab" :tabs="sections" label="设置分类" />
<SegmentedTabs v-model="active" class="lms-tabs" compact :tabs="tabs" label="活动分类" />
```

两者都接收 `{ id, title }[]`、`v-model` 和必填的 `label`（`aria-label`），并支持方向键 / Home / End 切换。页面不再用 `:deep()` 覆盖标签栏颜色、底色或间距。选中态保持实色浅底（`--accent-soft` / 白底），不要换成半透明色，否则壁纸纹理会穿过文字。

## 控件底色与透明度

有壁纸时，页面和卡片是半透明的（`--panel`），控件不能再用纯白盖住壁纸，也不能各写一套颜色。统一使用下面的 token（定义在 `style.css`，`desktop-shell.css` 的 `.desktop-shell.has-wallpaper` 负责切换）：

| token | 无壁纸 | 有壁纸 | 用途 |
| --- | --- | --- | --- |
| `--control-bg` | `#fff` | `rgba(255,255,255,.55)` | 按钮、输入框、选择器、周次选择 |
| `--control-bg-subtle` | `#f7f8fa` | `rgba(255,255,255,.4)` | 页头搜索框等次要输入 |
| `--control-border` | `#d4d7dc` | `rgba(15,23,42,.1)` | 控件描边；壁纸下仍要能看出边界 |
| `--control-sunken` | `#f1f2f4` | `rgba(15,23,42,.05)` | 分段标签的轨道 |

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

## 检查

前端改动后运行 `npm run build`、`npm test`、`npm run format:check`。用 `grep -rn '<button' src/components` 确认没有遗漏的裸 `.btn` 类；`cargo` 相关检查仅在后端同时改动时需要。
