# 进度交接：v0.6 前端组件套件（2026-10-31 已产出）

这份文件只记录**当前未完成的工作状态**，供下一个模型接手。规则以 [CONVENTIONS.md](CONVENTIONS.md) 为准，需求以 [need-do.md](need-do.md) 为准，组件清单以 [docs/design/campus-v0.6/LAYOUT.md](../docs/design/campus-v0.6/LAYOUT.md) §4.2 为准。

## 分支与工作区

- 分支 `dev-v0.6.0`，未推送的提交 3 个：`bca6c4a`（v0.6 校园清单 / 规格 / 排版文档）、`6092c2c`（前端思源学堂时间按校园时区显示）、`3d17353`（后端同样处理）。
- 工作区**未提交**：LAYOUT §4.2 的全部组件、校园工作区接线（`CampusView`、`App.vue`、`desktop-shell.css`、`config.rs` 的 `campusRailCollapsed`）、域函数与单测、三个 token、组件画廊、[UI_COMPONENTS.md](UI_COMPONENTS.md) 的套件说明，以及 `package.json` / `package-lock.json` 中精确版本的 `"pdfjs-dist": "6.4.299"`。
- 2026-10-31 用户明确要求“直接全量接入”：按现有 `docs/design/campus-v0.6/LAYOUT.md` 作为视觉依据，把已产出的组件接进 `CampusView` 的各校园分区，并补齐可用的后端查询。**评教页视觉仍未获准**（LAYOUT §9.11），本次没有改动 `CampusEvaluations.vue` 的外观。

## 已确定的实现口径（不要再重新讨论）

1. 组件套件 + 仅开发环境可见的 gallery 页保留；2026-10-31 起已按用户授权接入校园页面，接线依据 LAYOUT §2.1 / §3 / §9。
2. gallery 里的示例数据必须标注"示例"。
3. 组件位置：通用件放 `src/components/ui/`，校园业务件放 `src/components/campus/`。
4. 不新增假域名、假版本、伪造插件。

## 接入结果（2026-10-31）

`CampusView` 已改成工作区：左侧 `CampusRail`（待办 / 通知 / 学习 / 学业 / 生活），右侧 `SectionHeader` + 分区内容；`App.vue` 为校园路由加 `is-wide` 满宽，并把 `campusRailCollapsed` 存进配置（Rust / TS 两边都有字段）。

已接线的分区与真实查询：

| 分区 | 组件 | 后端查询 |
| --- | --- | --- |
| 待办 | `CampusTodos` | `lmsOverview`（当前学期作业） |
| 通知 | `CampusNotices` | `noticeChannels`、`notices` |
| 思源学堂 | `CampusLms` | `lmsCourses`、`lmsActivities`、`lmsActivity`、`lmsSubmissions` |
| 资料库 | `CampusLibraryFiles` | `lmsOverview`（当前学期资料附件） |
| 成绩 | `CampusGrades` | `terms`、`grades` |
| 考试 | `CampusExams` | `terms`、`exams` |
| 空闲教室 | `CampusRooms`（原页面） | `roomOptions`、`rooms` |
| 全校课程 | `CampusCourses` | `terms`、`courses`（空关键词取整学期） |
| 校历 | `CampusCalendar` | `calendar` |
| 校园卡 | `CampusCard` | `card`、`cardTransactions`（直连） |
| 图书馆 | `CampusLibrary` | `libraryFloors`、`libraryAreas`、`librarySeats` |
| 场馆 | `CampusVenue` | `venueList`、`venueOrders`、`venueSlots` |
| 考勤 | `CampusAttendance` | `attendanceRecords`、`attendanceStreams` |
| 体测 | `CampusFitness`（原页面） | `fitness`、`fitnessScore`（直连） |
| 评教 | `CampusEvaluations`（未改视觉） | `evaluations` + 既有写流程 |
| 学籍档案 | `CampusProfile`（原页面） | `profile` |

通知正文、应用内交作业、图书馆选座、场馆预订也都没有接入写接口，页面会明确说明。图书馆与场馆都会在打开时自动登录（图书馆走 WebVPN 端口映射，场馆直连其公开 HTTP 入口），校园卡 / 体测点击时先建立直连会话。

## 已产出组件（LAYOUT §4.2 全量）

`CampusRail`、`SectionHeader`、`SplitView`、`ListItem`、`StatTile`/`StatRow`、`StatusBadge`、`DueLabel`、`FilterBar`、`DataTable`、`FileRow`、`DropZone`、`FilePreviewDialog`、`WritePreviewPanel`、`ReaderView`、`SlotMatrix`、`SeatMap`、`Skeleton`、`InlineAlert`、`MoreMenu`、`MiniBars`/`Sparkline`。

配套完成：`src/style.css` 三个 token；`src/domain.ts` 校园时间 / 截止 / 更新 / 金额 / 文件大小 / 成绩 / 待办分组 / 考场与节次 / 考勤统计 / 校园卡统计 / 日期键等纯函数及 56 条单测；[UI_COMPONENTS.md](UI_COMPONENTS.md) 已登记全部组件、token 与接线注意。组件画廊（`KitGallery.vue`，侧栏「组件」入口）已在 2026-11-08 验收后移除，之后构建的就是真实发布的界面。

## 已确认的技术细节（避免重查）

- 设计 token、共享类（`src/ui.css`）、现有 UI 组件清单、`useSlidingIndicator`、对话框结构、剪贴板降级写法，均已在上一轮梳理完毕，直接复用。
- `pdfjs-dist@6.4.299` 用 legacy 构建 `pdfjs-dist/legacy/build/pdf.mjs` + `pdf.worker.min.mjs`；`getDocument` 的 `cMapUrl` / `standardFontDataUrl` / `wasmUrl` 必须以 `/` 结尾；要支持 CJK 与标准字体需把 `cmaps`、`standard_fonts`、`wasm/` 资源拷进 `public/`。
- CSP 需增加 `worker-src 'self'`（pdf.js worker）与 `media-src https://review-class.xjtu.edu.cn`（评教视频），这两项属于**接线阶段**的改动，不在本次组件套件里。
- 状态色已用 dataviz 校验：success `#2c8653` 与 warning `#a15c00` 在红绿色弱下 ΔE 7.2（WARN），所以状态**必须**图标 + 文字 + 2px 间隔做二次编码；考勤堆叠条顺序把 danger 与 warning 隔开（正常 → 迟到 → 请假/灰 → 缺勤）。
- 图表规则：纯 SVG、单轴、2px 线、≥8px 标记、≤24px 柱、数据端 4px 圆角、2px 表面间隙、文字用文字色而非系列色、hover/focus tooltip、提供"查看数据"表格、≥2 系列配图例。

## 2026-10-31 第二轮：自动登录、缓存与 WebVPN

用户指出分页打不开、数据每次重拉、思源学堂必须手动开页面登录且不稳定。本轮改动：

- **思源学堂自动登录**：后端错误文案统一为“请先完成思源学堂登录”，前端 `missingSession` 同时匹配“请先打开 / 请先完成 / 失效 / 未登录”，`siteQuery` 第一次查询即可触发现有密码登录，无需先打开校园网页。LMS 客户端的请求模式跟随账号（`direct` / `webvpn`），不再硬编码直连。
- **图书馆 / 场馆自动登录**：WebVPN 现在支持带端口的明文 HTTP 映射（`http-8086`），`refresh_library_session` 不再拒绝 WebVPN，登录时用账号已有的 VPN 会话作为种子；图书馆页面取消“仅校园网”禁用。场馆没有 WebVPN 映射，但它是公开 HTTP 站点，改为两种网络下都直接尝试其 OAuth 登录，不再在 WebVPN 下禁用。
- **读写缓存**：后端 `school_query` 在未强制刷新时，按查询类型 TTL（学期 24h、成绩/考试 10min、通知 5min、座位 30s、时段 60s、校园卡 3min 等）直接返回未过期的缓存；页面刷新按钮走 `force=true` 绕过 TTL。缓存仍按账号目录隔离。
- **启动预取**：新增 `src/campusPrefetch.ts`，账号就绪后在后台拉取学期、成绩、考试、校历、空闲教室选项、通知各栏目、思源学堂课程、图书馆楼层；账号登录成功后强制重拉一次。失败静默，页面打开时仍会解释原因。
- **不稳定重试**：只读查询遇到“无法连接 / 暂不可用 / 超时 / 非预期数据”时最多重试 2 次（0.5s、1.2s 退避）；会话失效仍只刷新一次。
- **缓存键修正**：通知的缓存键改用 `cacheName('notices', id)`，此前写入与读取的键不一致，首屏缓存一直没生效；图书馆座位、场馆时段、思源学堂活动详情/提交记录补上了缓存键。

当时仍未接入：待办与资料库（后端无跨课程聚合接口，参考项目也没有）。评教视觉未改。

## 2026-10-31 第三轮：默认视图、课程关注与学期过滤

用户要求打开页面不要空空如也，课程不要一次性铺开全部学期，并要能快速回到自己关注的课程。

- **校园页记住上次分区**：`CampusView` 把最后成功打开的分区写入 `localStorage`（`campus.lastSection`），下次进入直接打开；记录失效或分区被禁用时回到通知。
- **思源学堂默认打开课程**：课程加载后优先打开上次选择的课程（按账号保存在 `campus.lms.course:<学号>`），没有记录时按课程 `start`/`end` 判断当前学期并打开其中第一门；活动默认停在“作业”分类。课程列表也只在没有选中课程时显示当前学期。
- **课程不再一次铺开**：表头拆成“学期 + 课程”两个选择器，课程选择器只列当前学期（学期分组本身只由有课程的学期生成）；点击学期会直接打开该学期第一门课，不会停在空页。
- **关注课程标签**：当前课程旁有星标按钮，关注后课程名以标签形式出现在页面上方，可跨学期一键跳回；关注列表按学号保存在本地，不进入仓库。
- **学期过滤**：成绩的学期下拉只保留成绩表中实际出现过的学期；考试、全校课程隐藏尚未开始的学年；体测隐藏未来学年。班级/日期/节次变化时空闲教室自动重新查询，首次进入也会自动查一次，不再只显示表单。
- 新增 `currentAcademicStart()` 纯函数与单测（8 月切换学年，1–7 月仍属上一学年）。

## 2026-10-31 第四轮：待办与资料库

用户要求先把待办和资料库做出真实数据。

- **后端聚合**：新增 `Query::LmsOverview`。先取课程，再按开课日期判断当前学期；LMS 多数课程的 `end_date` 为空，所以没有“正在运行”的课程时，取最近一个开课日期 cohort（±30 天）而不是扫描全部历史学期。并行读取该 cohort 的活动列表，再并行读取作业/资料活动详情，一次返回 `rows`（未提交且未关闭的作业）与 `files`（资料活动附件，按 upload id 去重），同时返回出现最多的学期标签与失败请求数；有请求失败时页面显示“列表可能不完整”，不会静默丢数据。
- **性能**：课程与活动请求按 6 个一组并发；在真实账号上当前学期为 11 门课，单次聚合约 14–17 秒（此前全量 58 门课约 45 秒）。后端缓存 300 秒，两个页面共用 `lmsOverview` 缓存；前端 `campusOverview.ts` 还会合并同时进行的同一请求，切换页面不会重复扫描。
- **前端**：`CampusTodos` 按逾期 / 今天 / 明天 / 本周 / 以后 / 无截止分组，显示课程、小组作业、提交次数和截止倒计时；`CampusLibraryFiles` 按课程分组，支持搜索文件名 / 资料名 / 课程，打开和下载由用户手动触发。两者都先读本地缓存，首次加载有明确说明，首次打开不再是空白页。
- **导航**：待办与资料库取消禁用；账号切换或重新登录时清空前端内存聚合，后端缓存本来按账号隔离。
- **测试**：新增 `current_course_group` 单测（运行中课程、空结束日期、未开学、无日期）与两个 ignored 真实账号探针（课程日期分布、聚合计数）。

## 验收状态

- `npm run build`：通过（vue-tsc + vite；生产包不含画廊与 pdf.js）。
- `npm test`：56 条全部通过。
- `cargo test`：127 条通过、12 条 ignored（含新增的课程分组单测与真实账号探针）。
- `npm run format:check`：通过；`src` 内已无裸 `class="btn"`。
- **真实账号已验证**：`live_lms_aggregate_probe` 在当前账号上得到 58 门课程中的 11 门当前学期课程、11 项待交作业、35 个资料文件（6 门课），单次约 17 秒；只打印计数与日期，不打印课程名、文件名或凭据。
- **未做**：真机逐项点检各分区、真实 PDF 验证、用户验收；未验证 Windows。
- 评教页视觉仍未获准；本次只把它放进工作区，没有改它的外观。

## 下一步

1. 真机（真正的 Tauri App）逐分区点检：工作区宽度、侧栏折叠、WebVPN / 直连切换、空状态与错误态。
2. 接线 `FilePreviewDialog` 时补 CSP `worker-src 'self'`，把 pdf.js 的 `cmaps/`、`standard_fonts/`、`wasm/` 拷到 `public/` 并配置路径。
3. 写操作页面严格保持“预览 → 用户明确确认 → `confirm=true`”，`unknown` 不重试。
4. 不要自行提交或推送；用户没要求就不 commit。

## 其它挂着的事项（与本任务无关但未完成）

- PR #1（dev 合入 main 的流程改动）等 `tested` 标签。
- 图书馆 / 场馆的真实接口探测需校园网环境。
- 参考图获批后才能动校园页面视觉。
