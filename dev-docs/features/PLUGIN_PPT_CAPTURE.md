# 回放截取插件方案（内置）

更新时间：2026-11-06。状态：**方案已确定；截取核心已对齐上游 v3.1.0，入口、进度保存、整理与三种导出已实现；视觉未验收，真机截取与导出待验证**。用户已在 2026-11-05 要求把 [XJTU-PPT-Capture-Extension](https://github.com/MeowSaint/XJTU-PPT-Capture-Extension) 作为插件集成，并明确“插件直接加”，所以清单、页面、采样与导出按现有组件先落地；页面视觉仍需用户验收，见 §10。2026-11-06 用户指出与上游差距大、入口不对、功能不稳，已按“只放在课时详情”和“分批对齐上游核心”调整，见 §2 与 §11。Windows / WebView2 未验证。

## 1. 结论

- 做成**内置插件** `cn.xjtu.ppt-capture`，随 App 版本发布，与课表、畅行校园同级；不是校园页里的一个按钮，也不是第三方远端插件。
- 只复用扩展的**截取与导出核心**（跳转采样、翻页检测、去重、框选、PDF / ZIP 导出）；不移植 Chrome 专有部分（`permissions`、`tabs`、`webRequest`、`declarativeNetRequest`、`getDisplayMedia`）。
- 网络入口仍在 Rust 后端：复用已经实测的 `player-url → embed-token → captures` 链路，只把 `review-class.xjtu.edu.cn` 的预签名 `download_url` 交给前端播放器。扩展的“嗅探播放器请求”整段去掉。
- 路由仍由 App 持有，插件只声明 `page.type = pptCapture`；外部插件不能使用该页面类型。

## 2. 入口位置（2026-11-06 用户决定）

首版把入口放在侧栏「插件」分组，用户验收后认为位置不对：截取不是独立工具，而是「看回放」的延伸。现在改为：

- 思源学堂 → 课程回放 → 课时详情 → 「回放」分组里的 **截取讲义**；
- 进入后带着该课时（`activity` / `course` / 标题）打开截取页，页面仍由插件提供、仍可被停用；
- 清单用 `sidebar: false` 关闭侧栏入口，插件中心里仍能看到、启停。

| 其他方案             | 为什么不用                                                                                                                   |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| 塞进课时详情内联展开 | 截取是多步骤工作流（解析视频 → 采样 → 挑图 → 导出），塞进详情会把 LAYOUT §9.3 撑开；跳到独立页面但保留返回课时上下文的入口。 |
| 第三方远端插件       | 当前协议只支持声明式导航，没有可执行运行时；不引入通用运行时是 2026-10-29 的明确决定。                                       |
| 独立小工具 / sidecar | 增加双平台签名、更新和进程管理成本；前端已有 canvas 与视频解码能力。                                                         |

## 3. 插件清单（已实现）

清单已写入 `plugins/cn.xjtu.ppt-capture/plugin.json`，由 `plugins::list()` 用 `include_str!` 编入。

```json
{
  "schemaVersion": 1,
  "id": "cn.xjtu.ppt-capture",
  "name": "回放截取",
  "version": "0.1.0",
  "description": "从思源学堂课程回放中截取讲义页面，导出为 PDF 或图片包",
  "icon": "presentation",
  "page": { "type": "pptCapture" },
  "widget": false,
  "sidebar": false
}
```

`sidebar: false` 只关闭侧栏入口；插件中心、启停和路线不变。

- `id` 加入 `src-tauri/src/plugins/manifest.rs` 的内置 ID 集合与保留 ID 校验，外部清单不得占用。
- `Page` 新增 `PptCapture` 变体；`validate(false)` 对非内置清单返回“回放截取仅供内置插件使用”，与 `timetable` / `campus` 一致。
- `description` 只用真实能力，不提“自动下载视频”“自动翻页抓全部课程”等未实现行为。
- 课时详情按钮使用 `Presentation`（lucide）图标；`sidebar: false` 后该插件不再出现在侧栏。

## 4. 后端接口

### 4.1 新命令 `lms_replay_videos`

```rust
#[tauri::command]
async fn lms_replay_videos(app: AppHandle, activity: u64) -> Result<ReplayVideos, String>;
```

不复用 `school_query`：预签名地址带有效期，不能进磁盘缓存，也不应出现在 `cache/` 里。

返回给前端的结构（只含播放所需字段）：

```rust
struct ReplayVideo {
    label: String,       // INSTRUCTOR / ENCODER 等原始标签
    size: u64,
    mute: bool,
    best_audio: bool,
    download_url: String, // 预签名，只在内存与 IPC 中出现
}
struct ReplayVideos {
    activity: u64,
    title: String,
    videos: Vec<ReplayVideo>,
}
```

- 先按 `activity` 读详情，确认 `type == "lesson"` 且 `replayCode` 非空；再用 `player-url` 的 token 换 RMS bearer（`Authorization` 头），请求 `captures/{replay}?lesson_activity_id=…`。
- 兼容 `data.lesson_videos` 与顶层 `lesson_videos`，字段兼容 `download_url` / `downloadUrl`、`is_best_audio` / `isBestAudio`（真实返回已确认过）。
- `download_url` 必须校验：HTTPS、host 恰为 `review-class.xjtu.edu.cn`、不含账号密码；否则丢弃该路视频。
- bearer 令牌、原始响应、完整地址都不得写日志、错误文案、缓存或截图；错误只报“回放视频服务返回 HTTP xxx / 回放视频响应格式异常”。
- 账号代次检查照旧：命令执行期间切换账号，结果丢弃。

### 4.2 导出落盘命令

前端生成 PDF / ZIP 后需要一个写文件入口。当前没有 `plugin-fs`，capabilities 只开了 `dialog:allow-save`，因此新增：

```rust
#[tauri::command]
fn save_export(path: String, bytes: Vec<u8>) -> Result<u64, String>;
```

- 路径只能来自 `save()` 对话框的返回值；拒绝目录、空路径和超过 256 MB 的结果。
- 先写同目录临时文件再原子改名，失败不留半个文件。
- 大小上限与截取帧数上限一致，超限报错，不静默截断。

### 4.3 截取进度命令（2026-11-06）

进度与图片属于应用缓存，按约定放在 `~/.xjtu-launcher/cache/` 下，不进 webview 的 IndexedDB：

```
~/.xjtu-launcher/cache/<账号哈希>/capture/<activity>/
├── session.json     扫描设置、断点、疑似过渡段
├── pages.json       页码、时间、尺寸、签名
└── pages/<id>.jpg   保留的幻灯片原图
```

| 命令                    | 用途                              |
| ----------------------- | --------------------------------- |
| `capture_session_load`  | 读取本课时的设置、断点与页列表    |
| `capture_session_save`  | 写入设置、断点与疑似段            |
| `capture_page_add`      | 追加一页（base64 JPEG），返回页号 |
| `capture_pages_read`    | 批量读取页面原图（恢复网格用）    |
| `capture_pages_delete`  | 删除页面及其图片                  |
| `capture_pages_order`   | 保存拖动后的页序                  |
| `capture_session_clear` | 从头扫描前清空本课时              |

- 账号目录用活动账号哈希，与校园缓存一致；删除账号时随 `cache::purge` 一并清除。
- 单页 JPEG 上限 4 MB，最多 1000 页；设置数据上限 1 MB。
- 只接受主窗口调用（`main_only`）；课时号与页号都是数字，不拼接用户输入路径。
- 不保存预签名地址、Cookie 或令牌；会话里只有课时、设置、断点和用户保留的图片。

## 5. 前端数据流

```
插件页 → 课程列表（沿用 lmsCourses 缓存）
      → 该课程 type=lesson 的活动（沿用 lmsActivities）
      → 选中活动 → lms_replay_videos
      → <video> + <source type="video/mp4" src=presigned>
      → 跳转采样 / 补查 / 确认 / 去重
      → 保存进度与页图（cache/<账号>/capture/<activity>）
      → 缩略图挑选 / 排序 / 清理相似页 / 补截
      → 内存中生成 PDF 或 ZIP
      → save() 选路径 → save_export
```

要点：

- 播放器必须写成 `<video><source type="video/mp4"></video>`。响应没有 `content-type` 且是 `attachment`，直接 `<video src>` 会报 `MEDIA_ERR_SRC_NOT_SUPPORTED`（已实测）。
- 后端按课返回 1–2 路视频（实测 `INSTRUCTOR` 是教室摄像头、约 960 MB，`ENCODER` 是屏幕录制、约 487 MB，幻灯片只在后者里）。默认打开标签含 `ENCODER` 的一路，没有时退回 `bestAudio`，再退回第一路；标签只照实显示，由用户手动切换。
- 预签名过期：重新调用 `lms_replay_videos` 重新解析（最多 2 次），不自动重试导出。
- 切换账号、删除账号、页面关闭时立即销毁 `<video>`、canvas 和帧数据，丢弃未完成请求。

## 6. 截取核心（已对齐上游 v3.1.0）

2026-11-06 按上游 v3.1.0 重写，纯逻辑放在 `src/features/plugins/capture/`，视频读写由页面注入，便于单测：

1. **稳定跳转**：设置 `currentTime` 后等 `seeked` 与 `readyState >= 2`，再等 `requestVideoFrameCallback`（150 ms 回退）画帧。单次跳转 20 秒超时；超时重新解析回放地址并重挂 `<video>`，最多恢复 2 次，不把“地址过期”误报成整节课失败。
2. **采样**：三种方式——均衡每 2 秒、细查每 0.5 秒、快速每 8 秒。两次粗采样间隔超过 0.55 秒时，在中间按 0.5 秒补查，避免漏掉短页面。
3. **比对**：帧缩到 192×108 RGBA，逐通道去偏（|偏移| ≤ 8），像素差 ≥ 24 记为变化；变化像素占比超过“新页差异”（默认 1.2%）且平均差不低于 `max(0.25, 阈值/2)` 才算换页候选。
4. **确认与疑似段**：候选点后 0.25 秒再采一帧；仍在变化说明是过渡动画，记入“画面不稳定”区间（前后各 0.5 秒，重叠合并），不保存中间态。
5. **去重**：确认稳定后才与本次已保留的所有页面比对；命中则丢弃。去重放在确认之后，过渡页不会挡住后面的真实页面。
6. **空白/纯色检测**：方差过低时跳过，不覆盖上一张。
7. **框选与上限**：默认全画面，可拖动 ROI；单次最多 1000 页，达到上限停止并明确提示，不自动降质。
8. **断点续跑**：每个粗采样后写入断点；只有扫描方式、阈值、去重与 ROI 都与上次一致时才允许“继续截取”，否则要求从头扫描并二次确认清空。

canvas 跨域：`review-class` 返回 `access-control-allow-origin: *`，实测 `drawImage` 不污染 canvas；若未来响应变化导致 `toDataURL` 抛安全错误，直接报“视频不允许截取”，不要回退到截图工具或系统录屏。

## 7. 导出与存储

- 格式：PDF（每帧一页，按帧原始比例）与 ZIP（JPEG 原图）。两者都在前端内存生成，PDF 手写、ZIP 手写，不新增运行时依赖。
- 路径：保存对话框默认落在可见目录 `~/xjtu-launcher-file/download/回放截取/<课程>/<日期 活动标题>.pdf`，由用户最终确认；写盘前只显示 `~/…`，不出现账号名。
- 尺寸估算：1280×720 JPEG 质量 0.85 约 100–250 KB/页；50 页约 5–12 MB；一门课全部回放约 30–100 MB。`download/` 属于用户文件，用户可自行清理，不纳入迁移逻辑。
- 截取进度与页面原图放在 `~/.xjtu-launcher/cache/<账号哈希>/capture/<activity>/`（见 §4.3），与校园缓存同一账号隔离规则：切换账号看不到上一人的内容，删除账号时一并清除。不写 IndexedDB 或 localStorage，避免散落到 webview 配置目录。
- 恢复网格时按页号批量读取原图，单页上限 4 MB；读取失败只丢该页，不影响其余缩略图。
- 预览用 `blob:` URL，`img-src` 已允许 `blob:`；用完立即 `revokeObjectURL`。

## 8. 安全、隐私与 CSP

- 只读：不提交作业、不写评教、不改课程设置；所有请求都走现有 LMS 会话。
- 预签名地址视为短期凭据：不写日志、不进缓存、不进错误文案、不进 `config.json` / `storage.json`，不做截图素材。
- 截取结果是课程内容：只写用户选择的位置，不上传、不遥测、不进入 Agent 上下文。
- 界面与文档必须保留版权提示：仅供本人课程学习使用，不得传播；不做批量自动抓取。
- `src-tauri/tauri.conf.json` 的 CSP 增加 `media-src https://review-class.xjtu.edu.cn blob:;`。不要放宽 `default-src`、`connect-src` 或 `script-src`。
- 主窗口 IPC 权限不变；校园登录窗口仍然没有 IPC。

## 9. 体积与性能预算

- 新增前端代码预计 1200–1800 行；打包后约 60–120 KB（gzip），不引入 `hls.js`（回放是 MP4 直链）。
- Rust 侧只增加一个命令与响应结构，无新 crate。
- 单帧 RGBA 1280×720 约 3.5 MB；1000 页不能全部常驻，按 JPEG Blob 保存，超限前提示先导出或删除；缓存目录里的原图单页上限 4 MB。
- 跳转采样比实时播放快得多，但每次 seek 有网络延迟；并发固定为 1，避免把预签名地址打到限速。

## 10. 界面（已实现，视觉待验收）

用户已同意“作为插件集成”，并在 2026-11-05 明确“插件直接加，你应该知道加在哪里”，因此页面直接按现有桌面壳实现，**没有单独出参考图，视觉也尚未验收**。按 [约定](../../AGENTS.md)，后续任何视觉调整仍需先给方案并获准，不得按本文的结构图自行改版。

2026-11-06 用户明确入口位置错误，已改为课时详情内的「截取讲义」，见 §2；截取页本身仍是同一页面，去掉侧栏入口。

实现用的是现有组件：课时详情按钮 `Presentation` 图标、内容区、`SectionHeader`、`UiButton`、`MenuSelect`、`InlineAlert`、`EmptyState`，颜色只取 `src/styles/campus.css` 的令牌。

信息结构（结构说明，不是视觉稿）：

```
回放截取                                  [刷新]
课程 [下拉]   回放 [列表]

┌ 播放区 ─────────────────────────────┐
│ <video>                              │
│ 时间轴 / 当前视频: INSTRUCTOR · 960MB │
└──────────────────────────────────────┘

扫描方式 [均衡|细查|快速]  新页差异 [1.2%]  区域 [全画面|框选]  [开始截取]
进度 12/320 · 已保留 18 页
n 段画面不稳定 → [回看此段] [细查此段]
[缩略图网格：勾选 / 删除 / 撤销 / 清理相似页 / 拖动排序 / 大图预览]
[导出 PDF]  [导出讲义 PDF]  [导出图片包]
```

状态：未选课程、回放为空、视频解析失败、地址过期、采集中、达到帧数上限、导出中、保存成功。全部用现有提示组件，不新造一套。

## 11. 分阶段实施

| 阶段 | 内容                                         | 验收                                                                                                                         |
| ---- | -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| P0   | 参考图 + 用户批准                            | 用户明确同意                                                                                                                 |
| P1   | `ReplayVideos` 生产实现、host 校验、单元测试 | ✅ 2026-11-05：Rust 141 项测试通过；真实账号探针取到 960 MB 视频，Range 206、CORS `*`；未输出令牌与地址                      |
| P2   | 插件清单、`Page::PptCapture`、入口、只读播放 | 🟡 代码完成；2026-11-06 入口改为课时详情「截取讲义」；真机打开回放并播放待验证                                               |
| P3   | 采样、补查、确认、去重、疑似过渡段           | 🟡 2026-11-06 对齐上游 v3.1.0，前端 78 项测试通过；真机截取一节课待验证                                                      |
| P4   | PDF / 讲义 PDF / ZIP 导出、`save_export`     | 🟡 三种格式与命名规则代码完成，写入走 `write_atomic`；保存后从访达打开待验证                                                 |
| P5   | 进度保存、续跑、整理、账号切换               | 🟡 代码完成：缓存目录持久化（Rust 3 项测试）、拖动排序、撤销删除、相似页清理、大图预览、手动补截；换账号后旧帧清空待真机验证 |
| P6   | 本地视频缓存、整课批量提取与 PDF 包          | ⬜ 第二期，未开始                                                                                                            |

Windows / WebView2 未验证，不能写成“双平台可用”。真机验证需要可用的屏幕截图权限；2026-11-06 该权限不可用，界面与导出尚未实机验收。

## 12. 许可与版权

- 上游 MIT，v3.1.0（初版按 v2.5.2 移植，2026-11-06 对齐）。派生文件 `src/features/plugins/capture/frames.ts`、`scanner.ts`、`seek.ts`、`naming.ts`、`export.ts` 已加版权头；`THIRD_PARTY_NOTICES.md` 记录来源、版本、许可与逐文件修改说明；README 致谢区已同步。
- 只移植算法与导出逻辑，不复制其扩展清单、Chrome API 调用、网络拦截和权限申请。
- 上游若变更许可证，先停下评估，不直接跟随。

## 13. 待确认

1. 默认扫描方式与最大页数是否合适（当前均衡 2 秒 / 1000 页；细查 0.5 秒、快速 8 秒）。
2. 默认导出格式（原始 PDF / 讲义 PDF / ZIP）与文件名规则。
3. 是否需要“仅截取幻灯片区域”的自动检测；首版只做手动画框。
4. Windows 上预签名 MP4 的 Range 与 canvas 行为，需实机验证。
5. 实时共享画面（`getDisplayMedia`）不实现；如用户需要，另立需求。
