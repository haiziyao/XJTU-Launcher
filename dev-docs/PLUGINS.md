# 插件格式与插件中心

插件中心只展示**随应用发布的内置插件**。清单在构建时编入二进制，没有远端市场、下载、安装、卸载或本地导入（2026-11-08 用户决定）。

## 插件中心

- 侧栏入口名为「插件中心」，页面分「校园广场」「拓展发明」「创新小港」「已安装插件」四个分区。
- 每行是插件名称、右侧一行简述和启用开关；单击行展开作者、版本、分类与详细描述，启用后可从这里打开插件页面。
- 只有启用开关会改动配置；`disabledPlugins` 记录关闭的插件 id，侧栏入口跟随启用状态。
- 默认启用：课程表、畅行校园、Agent 设置、主页主题、回放截取；默认关闭：桌面宠物、字体美化。
- 旧配置里的 `pluginServer`、`checkPluginUpdates` 与桌面宠物字段只读取一次，用于迁移，之后不再写入。

## 清单格式

每个内置插件位于 `plugins/<id>/plugin.json`，由 `src-tauri/src/plugins.rs` 通过 `include_str!` 读取并校验。

```json
{
  "schemaVersion": 1,
  "id": "cn.xjtu.timetable",
  "name": "课程表",
  "version": "0.1.0",
  "category": "campus",
  "author": "haiziyao",
  "description": "查看学期课表与今日课程",
  "details": "从教务系统同步本学期课表，支持周次切换、课程编辑与自定义课程，主页会显示今天的课程。",
  "enabledByDefault": true,
  "icon": "calendar",
  "page": { "type": "timetable" },
  "widget": true
}
```

- `id`：必须是已登记的内置 id，未知 id 会被拒绝。
- `schemaVersion`：必须为 `1`。
- `category`：`campus`（校园广场）、`invention`（拓展发明）或 `lab`（创新小港）。
- `author`：1–40 个字符；当前内置插件统一写 `haiziyao`。
- `name`：1–30 个字符；`description` 最多 300 个字符；`details` 最多 600 个字符。
- `enabledByDefault`：新配置的初始状态；用户切换后以 `disabledPlugins` 为准。
- `page.type`：`timetable`、`campus`、`agentSettings`、`homeTheme`、`pptCapture`、`desktopPet`、`fontBeautify` 或 `links`。`links` 目前没有内置清单使用。
- `widget = true`：在主页「我的插件」区显示入口，目前只有课程表。
- `sidebar`：可选，缺省为 `true`。设为 `false` 时插件仍可在插件中心启停，但不占侧栏入口；回放截取从「思源学堂 → 课程回放」进入。
- `icon`：预留图标标识。

清单文件最多 64 KiB。校验失败会导致应用启动报错，不会静默跳过。

## 插件设置

插件自己的设置不写进 `config.json`，而是保存在：

```text
~/.xjtu-launcher/plugins/settings/<插件 id>.json
```

- 读写都按插件做值域校验：宠物角色、横条风格、通知来源、窗口位置；字体区域的字体名、字号、字重、行高与字距。非法值逐项回退默认，不让整页失败。
- 损坏或超大的插件文件只影响该插件，不阻止应用和配置加载。
- 写入使用原子替换，并与桌面宠物的位置保存共用同一把锁。
- 「设置 → 存储」里的可见文件夹另有 `plugin/` 放用户自备的插件资源，随文件夹迁移一起搬走。

## 内置插件一览

| 插件 | 分区 | 页面 | 默认 |
| --- | --- | --- | --- |
| `cn.xjtu.timetable` 课程表 | 校园广场 | `timetable` | 启用 |
| `cn.xjtu.campus` 畅行校园 | 校园广场 | `campus` | 启用 |
| `cn.xjtu.desktop-pet` 桌面宠物 | 拓展发明 | `desktopPet` | 关闭 |
| `cn.xjtu.font-beautify` 字体美化 | 拓展发明 | `fontBeautify` | 关闭 |
| `cn.xjtu.home-theme` 主页主题 | 拓展发明 | `homeTheme` | 启用 |
| `cn.xjtu.agent-settings` Agent 设置 | 拓展发明 | `agentSettings` | 启用 |
| `cn.xjtu.ppt-capture` 回放截取 | 创新小港 | `pptCapture` | 启用，无侧栏入口 |

### 桌面宠物

从设置页移入插件（2026-11-08）。主窗口最小化后出现浮动桌宠和通知横条；关闭窗口仍然是关闭。设置页提供角色、横条风格、只保留桌宠、台词气泡与通知来源。可预览；宠物窗口只拥有自己的少量命令，不获得主窗口 IPC。

### 字体美化

五个区域各自设置字体、字号、字重、行高与字距：界面、标题、正文、课表数字、代码。改动立即预览并保存；关闭插件后恢复应用自带字体。字体名必须是安全字符，字号 8–48 px，字重按 100 取整，行高 1.0–2.5，字距 −1.0–5.0 px。

### 回放截取

从思源学堂课程回放中截取讲义页面，导出 PDF 或图片包。入口在「思源学堂 → 课程回放 → 课时详情 → 截取讲义」，清单用 `sidebar: false` 关闭侧栏入口。方案见 [回放截取插件方案](PLUGIN_PPT_CAPTURE.md)；真机截取与导出、Windows 端尚未验收。

## 后续扩展

第三方校园业务、真正的可交互插件页面、独立主页小组件及 Agent 工具是期望能力。2026-10-29 起**不再计划通用插件运行时**：插件由项目自己实现，随 App 版本发布。下载与卸载是否可行尚未评估，本轮不提供（2026-11-08）。
