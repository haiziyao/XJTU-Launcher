# 开发指南

更新于 2026-10-23。当前发布版为 0.3.3（2026-10-23 发布），`dev` 继续承载后续开发，已完成的代码与尚待验收的能力分别记录。

## 代码与分支

`main` 已整合校园登录、原生校园窗口、本地 auth.json、校园后台适配，以及本轮插件市场 / Updater 实现。README 面向使用者，开发资料集中在 `dev`。

`dev` 从整理后的 `main` 创建，两者当前应用代码相同。此后在 `dev` 或其功能分支推进工作；开发专用文档单独提交，向 `main` 同步功能时保持主分支面向使用者的目录结构。现有 `codex/*` 与 `feat/*` 分支保留历史。

本次只完成本地提交，由用户自行 push。待用户发布分支后，README 中的 dev 文档链接才会在 GitHub 上可访问。

## 开发环境

- Node.js 22.12+ 与 npm；依赖版本由 `package-lock.json` 固定。
- Rust 工具链；依赖版本由 `src-tauri/Cargo.lock` 固定。
- macOS：Xcode Command Line Tools。
- Windows：Visual Studio C++ Build Tools 与 WebView2。

```sh
npm ci
npm run dev
```

`npm run dev` 启动原生 Tauri 窗口，Vite 仅为开发资源服务。单独 `npm run dev:frontend` 不能替代客户端验收。

macOS 构建独立 App：

```sh
npm run app:build -- --bundles app
open "src-tauri/target/release/bundle/macos/XJTU Launcher.app"
```

若 cargo 不在 PATH，先执行 `source "$HOME/.cargo/env"`。当前机器 Rust 位于 `/Users/bilibili/.cargo/bin/`。普通本地构建无需更新签名密钥。

## 目录与职责

| 路径 | 职责 |
| --- | --- |
| `src/App.vue` | 导航、配置持久化、账号刷新和插件入口 |
| `src/bridge.ts` / `src/types.ts` | 前端 IPC 边界与类型 |
| `src/components/ui/` | 通用行、分组、按钮、开关、标签、头像与空状态；按钮用法见 [按钮与通用控件](UI_COMPONENTS.md) |
| `src/style.css` / `ui.css` / `pages.css` / `desktop-shell.css` | token、共用控件、页面与窗口布局 |
| `src/market.ts` / `updates.ts` | 跨页面插件操作与更新状态 |
| `src-tauri/src/lib.rs` | IPC 注册、主窗口限制、任务锁和账号代次 |
| `auth.rs` / `accounts.rs` / `network.rs` | 登录状态机、校园账号与直连 / WebVPN |
| `vault.rs` | auth.json、迁移与原子持久化 |
| `campus.rs` / `campus_browser.rs` | 课表与独立校园网页窗口 |
| `school/` | 教务查询、评教与校园服务后端（学籍、思源学堂、图书馆、场馆） |
| `plugins.rs` / `market.rs` | 本地清单、远端目录、校验、安装及来源 |
| `updates.rs` | 官方 Updater 接线及任务互斥 |
| `plugins/` / `examples/` | 内置插件与声明式插件示例 |
| `tools/plugin-server/` | 可独立运行的目录服务与示例包 |
| `.github/workflows/` | 检查和手动触发的签名发布流程 |

## 持久化与运行边界

`~/.xjtu-launcher/auth.json` 保存校园账号、可选密码、会话与模型 API Key。按用户要求直接保存为本地 JSON，平时不用系统钥匙串。旧加密文件仅用于首次迁移；原文件与迁移失败的数据保留。不要把 auth.json 的真实内容放入调试输出。

普通配置、个人图片与安装插件在 Tauri 的应用目录。新配置字段必须提供默认值，以保留旧文件中的侧栏、插件和个人偏好。远端插件使用 `installed.json` 原子保存清单与来源；本地导入继续使用 `plugin.json`。

个人页的 MCP 模式由 `expose.rs` 提供，只监听 `127.0.0.1` 的 `POST /mcp`（默认 `39217`，端口来自配置 `mcpServerPort`，用户可改），不做请求认证。它复用 `agent::tools::builtin()` 的校园工具，按配置 `mcpServerEnabled` 与 `mcpServerDisabledTools` 启停和筛选；开关与端口在下次启动时恢复监听，端口被占用时保留原监听并给出原因。当前工具都是查询，后续可以加入写操作，但传输层不应写死“只读”；无论读写，都不要转发外部 MCP 服务、监听局域网或读取凭据，工具本身也不得返回密码、Cookie 或 API Key。

校园网页只复用账号所持有的会话，不能承诺所有学校网站免登录。窗口不得拥有启动器主窗口的 IPC 权限，账号变化时关闭旧窗口。后端查询必须固定账号快照并核对会话代次。

## 当前可用与未完成项

| 状态 | 范围 |
| --- | --- |
| 已有用户实测 | 校园账号登录、账号恢复和真实课表 |
| 已实现，待进一步验收 | 原生校园窗口逐站认证、插件市场操作、更新设置 |
| 后台已适配，页面未完成 | 本科成绩、考试、空教室、全校课程、校历、评教、考勤、校园卡、体测 |
| 页面已实现，待联调（0.3.3） | 学籍档案、思源学堂课程 / 分类活动 / 活动详情、图书馆楼层 / 区域 / 座位、场馆清单 / 时段 / 订单；思源学堂已用真实账号只读验证（含附件下载与回放地址），其余未联调 |
| 已实现，待用户验收（0.3.3） | 个人页 MCP 模式：本机接口（端口可改、失败回退原端口）、逐项开放工具、按分类展示名称 / 说明 / 可达性 |
| 待开发 | 研究生教务、LMS 作业提交与讨论 / 问卷内容、场馆与图书馆预订、通知、校园生活模块、Agent 运行时与社区同步 |
| 待部署 | 正式插件服务、应用更新端点、发布公钥和正式签名发布 |

Agent 需具备通用工具能力，并接入 MCP、Skills、持久对话及长期记忆。模型配置完成不代表 Agent 已能调用模型。现有 Agent 和评教新稿仍未获准，不能直接制作页面。

## 检查与本轮记录

较大一批修改后统一执行相关检查：

```sh
npm test
npm run build
npm run format:check
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

插件真实 HTTP 联调：先运行 `npm run market:serve`，再执行 `cargo run --locked --manifest-path src-tauri/Cargo.toml --example market_smoke`。测试使用临时安装目录，不接触用户凭据。原生 Cookie 隔离示例为 `campus_browser_smoke`，仅在需要核验校园窗口改动时运行。

插件市场轮次结果（历史）：前端 9 项测试通过，Rust 44 项通过、2 项联网探测默认忽略，Clippy 和格式检查通过。真实 HTTP 联调覆盖目录、下载、摘要失败、安装、来源保存和移除。独立 macOS App 已构建并打开，确认原生窗口及插件市场入口；Windows、正式签名更新和完整页面交互仍待验收。

0.3.3 校园后台轮次：Rust 84 项通过、6 项联网探测默认忽略，Clippy（`-D warnings`）与 `cargo fmt --check` 通过，`npm run build` 通过。新增覆盖账号 `sites` 校验、hello JWT 落地与 systemType 兜底、hello OAuth 的 HTTP 中转放行、LMS / 图书馆 / 场馆字段映射与错误页面识别、HTTP 来源白名单与后端白名单。四套新后端中 LMS 已完成真实账号只读联调，学籍、图书馆与场馆尚未联调（图书馆与场馆需校园网）；对应页面按同一校园子页样式实现，并在不可用时标注具体原因（WebVPN、未接后台或未实现），前端 15 项测试与构建通过。本批未替换独立发布的 App。

2026-10-23 修复体测查询：`Client::connect` 之前没有发送 User-Agent，CAS 返回统一认证登录页而不是服务票据，连接被判成功，随后体测接口提示“登录信息失效”。现在使用统一 UA，并在落地到 `login.xjtu.edu.cn` 时视为会话失效；已用本机真实账号完成只读学年查询（返回 6 个学年），成绩明细仍需页面验收。新增忽略式实测探针 `live_fitness_session_probe`。

2026-10-23 修复思源学堂登录：LMS 的 CAS 登录会先经过 `identity1.xjtu.edu.cn` 的 Keycloak broker（`/auth/realms/xjtu/...`）再回到 `login.xjtu.edu.cn`，该主机此前不在传输白名单，登录在重定向阶段即被拒（页面报“校园跳转地址不受支持”）。现在仅把该 HTTPS 主机加入白名单；它只中转登录跳转，不接收账号密码。已用本机真实账号实测：登录成功，课程 58 门、抽查课程活动分别 106 / 13 / 45 条。新增忽略式探针 `live_lms_session_probe`。

2026-10-23 按钮统一：所有文字按钮改由 `UiButton` 渲染，纯图标按钮统一为 `UiIconButton`（xs 20 / sm 24 / md 28，quiet / accent / primary / neutral / danger / inherit）。删除了页面里各自实现的 `history-new`、`icon-button`、`agent-icon-button`、`agent-send-circle`、`text-action` 等样式，`notice-close` 只保留外边距。页面级标签栏抽成 `PageTabs`（透明底、底部细线、主题色选中），应用设置、Agent 设置与插件市场共用；`SegmentedTabs` 只保留给分组内的紧凑筛选（思源学堂活动分类）。页头操作按钮统一为 `sm` 尺寸。控件底色改用 `--control-bg` / `--control-border` / `--control-sunken`：有壁纸时按钮、输入框、选择器和分段标签轨道变为半透明，原生 `<select>` 去掉系统白色胶囊并改用统一的 chevron；选中态仍是实色浅底。导航行、选择卡片、开关和选择器触发器仍使用原生按钮，与组件一起记录在 [按钮与通用控件](UI_COMPONENTS.md)。

2026-10-23 思源学堂内部页面：按用户要求改为“功能分类”而不是平铺活动。课程列表按学期（学年 + 学期）分组，新近学期在前；进入课程后顶部用分段标签切换分类，不再纵向堆叠所有分组。课程活动按类型分为作业、学习资料、课程回放、在线视频、直播课堂、讨论、问卷与其他活动，每类显示数量；点开活动进入详情页，显示时间、提交与成绩概况、活动说明（HTML 转为文本）、附件列表，并提供“打开”（校园窗口查看）与“下载”（另存为，流式写入所选路径）操作；录播课可取 `/api/lessons/{id}/player-url` 的回放地址并在校园窗口打开。新增 `Query::LmsActivity` 与命令 `lms_open_file`、`lms_save_file`：附件接口先 302 到 `media.xjtu.edu.cn`，只接受该 HTTPS 主机的跳转，下载上限 2 GB。已用真实账号验证：活动详情正常，回放地址可取得，附件实测下载 47 MB 视频成功。作业提交、讨论与问卷正文尚未实现。

2026-10-23 个人页 MCP 模式补强：按用户要求把端口改为可在页面编辑（默认 `39217`，范围 1024–65535；点“应用”先停旧监听再绑定新端口，绑定失败时回到原端口并显示原因，成功后才写入 `mcpServerPort`）。“查看工具”按课程与成绩、考勤、校园卡、体测、教室与校历分组；分类在 `expose.rs` 生成，页面与 `tools/list` 保持一致。“检查可达性”原来会带出“返回 73 个学期”这类查询细节，现在只显示“校园会话可用”或具体失败原因。工具当前都是查询，但文案与传输层不再写成永久只读，后续评教、预订等写操作可按同一接口加入。Rust 90 项测试（含新增的端口占用 / 释放复用测试）与 6 项联网探测（默认忽略）通过，Clippy（`-D warnings`）和 `cargo fmt --check` 通过。真实 App 验证：把 `mcpServerPort` 保存为 `39218` 后重启，监听切到 `39218`，`initialize`、`tools/list`（14 个工具）正常；改回 `39217` 后监听恢复。页面“应用”按钮的点击链路未完成端到端人工验证（macOS 自动化被其他前台应用抢占），留待验收。同日按反馈补上展开动画：MCP 详情和工具清单改为 `0fr → 1fr` 网格行高过渡（`src/pages.css` 的 `.expand`），关闭时不再瞬间弹跳，也不会在收起过程中闪出“服务未启动 / 重试”；关闭状态用 `inert` 屏蔽不可见控件。动画帧未截图验证，待用户手感确认。

## 字体

Windows 默认的微软雅黑在界面字号下偏粗，因此把 Noto Sans SC（SIL OFL 1.1）子集化后内嵌进应用：`src/assets/fonts/noto-sans-sc.woff2`（约 5.7 MB，覆盖常用拉丁、标点和 U+4E00–U+9FFF 汉字，可变字重 100–900）。样式表用 `@font-face` 注册为 `XJTU Sans SC`，仅在 `.platform-windows` 下排在字体栈首位；macOS 继续使用系统 SF Pro + 苹方。许可证文本 `src/assets/fonts/OFL.txt` 通过 `bundle.resources` 随应用分发。重新生成子集可用 `python3 -m fontTools.subset`，参数见提交记录。

## 下一步

1. 由用户验收插件市场与设置；部署正式插件服务与应用更新源时按 [发布指南](RELEASING.md) 配置。
2. 调整并确认校园查询 / 评教页面，接入已存在的 Rust 服务，逐项做真实账号验收。
3. 在校园网下用真实账号联调学籍档案、思源学堂、图书馆座位和场馆页面，随后按用户反馈调整。
4. 迁入其余校园能力，再落实 Agent、MCP、Skills 和记忆；前端仍先出参考图并获准。
