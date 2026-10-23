# 开发指南

更新于 2026-10-23。当前发布版为 0.3.1；`dev` 与 `codex/v0.3.3` 正在开发 0.3.3，已完成的代码与尚待验收的能力分别记录。

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
| `src/components/ui/` | 通用行、分组、开关、标签、头像与空状态 |
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

校园网页只复用账号所持有的会话，不能承诺所有学校网站免登录。窗口不得拥有启动器主窗口的 IPC 权限，账号变化时关闭旧窗口。后端查询必须固定账号快照并核对会话代次。

## 当前可用与未完成项

| 状态 | 范围 |
| --- | --- |
| 已有用户实测 | 校园账号登录、账号恢复和真实课表 |
| 已实现，待进一步验收 | 原生校园窗口逐站认证、插件市场操作、更新设置 |
| 后台已适配，页面未完成 | 本科成绩、考试、空教室、全校课程、校历、评教、考勤、校园卡、体测 |
| 页面已实现，待联调（0.3.3） | 学籍档案、思源学堂课程 / 活动、图书馆楼层 / 区域 / 座位、场馆清单 / 时段 / 订单；思源学堂课程 / 活动已用真实账号只读验证，其余未联调 |
| 待开发 | 研究生教务、LMS 作业与回放、场馆与图书馆预订、通知、校园生活模块、Agent 运行时与社区同步 |
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

0.3.3 校园后台轮次：Rust 81 项通过、5 项联网探测默认忽略，Clippy（`-D warnings`）与 `cargo fmt --check` 通过，`npm run build` 通过。新增覆盖账号 `sites` 校验、hello JWT 落地与 systemType 兜底、hello OAuth 的 HTTP 中转放行、LMS / 图书馆 / 场馆字段映射与错误页面识别、HTTP 来源白名单与后端白名单。四套新后端中 LMS 已完成真实账号只读联调，学籍、图书馆与场馆尚未联调（图书馆与场馆需校园网）；对应页面按同一校园子页样式实现，并在不可用时标注具体原因（WebVPN、未接后台或未实现），前端 15 项测试与构建通过。本批未替换独立发布的 App。

2026-10-23 修复体测查询：`Client::connect` 之前没有发送 User-Agent，CAS 返回统一认证登录页而不是服务票据，连接被判成功，随后体测接口提示“登录信息失效”。现在使用统一 UA，并在落地到 `login.xjtu.edu.cn` 时视为会话失效；已用本机真实账号完成只读学年查询（返回 6 个学年），成绩明细仍需页面验收。新增忽略式实测探针 `live_fitness_session_probe`。

2026-10-23 修复思源学堂登录：LMS 的 CAS 登录会先经过 `identity1.xjtu.edu.cn` 的 Keycloak broker（`/auth/realms/xjtu/...`）再回到 `login.xjtu.edu.cn`，该主机此前不在传输白名单，登录在重定向阶段即被拒（页面报“校园跳转地址不受支持”）。现在仅把该 HTTPS 主机加入白名单；它只中转登录跳转，不接收账号密码。已用本机真实账号实测：登录成功，课程 58 门、抽查课程活动分别 106 / 13 / 45 条。新增忽略式探针 `live_lms_session_probe`；课程活动详情、作业附件与回放仍未实现。

## 字体

Windows 默认的微软雅黑在界面字号下偏粗，因此把 Noto Sans SC（SIL OFL 1.1）子集化后内嵌进应用：`src/assets/fonts/noto-sans-sc.woff2`（约 5.7 MB，覆盖常用拉丁、标点和 U+4E00–U+9FFF 汉字，可变字重 100–900）。样式表用 `@font-face` 注册为 `XJTU Sans SC`，仅在 `.platform-windows` 下排在字体栈首位；macOS 继续使用系统 SF Pro + 苹方。许可证文本 `src/assets/fonts/OFL.txt` 通过 `bundle.resources` 随应用分发。重新生成子集可用 `python3 -m fontTools.subset`，参数见提交记录。

## 下一步

1. 由用户验收插件市场与设置；部署正式插件服务与应用更新源时按 [发布指南](RELEASING.md) 配置。
2. 调整并确认校园查询 / 评教页面，接入已存在的 Rust 服务，逐项做真实账号验收。
3. 在校园网下用真实账号联调学籍档案、思源学堂、图书馆座位和场馆页面，随后按用户反馈调整。
4. 迁入其余校园能力，再落实 Agent、MCP、Skills 和记忆；前端仍先出参考图并获准。
