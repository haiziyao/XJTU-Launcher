# 项目工作约定

本文件给 coding agent 提供**必须遵守的硬约束**。完整规则见 [约定手册](dev-docs/CONVENTIONS.md)，本文件不重复细节；两者冲突时以约定手册为准。

开始工作先读：[开发须知](dev.md)、[开发待办清单](todo_list.md)、[验收清单](acceptance_list.md)、[开发文档索引](dev-docs/README.md)、[开发指南](dev-docs/DEVELOPMENT.md)、[约定手册](dev-docs/CONVENTIONS.md)、[需求时间线](dev-docs/planning/need-do.md)。项目 skill 在 [skills/](skills/)（`xjtul-style`、`xjtul-plugin`、`xjtul-push`）。历史讨论在 `dev-docs/archive/`，不能用旧记录覆盖后来明确的用户决定。

## 硬约束

- **设计**：新前端必须先有参考图，且用户明确同意设计与开始实现。`docs/design/plugin-market-v2/` 的软件更新部分已获准，插件市场部分已由内置插件中心取代（2026-11-08）；`campus-agent-v1` 的 Agent / 评教页、最小化桌宠设计与 `style-v1` 风格系统**未获准**，不得按旧稿或设想直接改视觉。`docs/design/campus-v0.6/` 范围与信息架构已确认；版本 B 的导航、待办、课程资料版式已获准并实现（2026-11-02），其余页面参考图获批前不改视觉。工作台 v1、社区广场默认页、字体库已实现，视觉仍待用户验收。
- **技术栈**：Rust + Tauri 2 + Vue 3 + TypeScript，目标 macOS / Windows。校园功能先研究本地 XJTUToolBox，再独立适配；复用 GPL-3.0 代码前核对分发要求。代码按 [代码结构](dev-docs/architecture/CODE_STRUCTURE.md) 分层：Rust 命令进 `src-tauri/src/commands/`，后端领域模块按 `core/`、`identity/`、`campus/`、`school/`、`agent/`、`plugins/`、`features/`、`workbench/` 分组；前端页面与页面私有逻辑进 `src/features/<domain>/`，跨功能逻辑进 `src/shared/`，领域类型进 `src/types/<domain>.ts`，全局样式进 `src/styles/`。
- **凭据**：保存在 `~/.xjtu-launcher/auth.json`，不使用系统钥匙串。密码、Cookie、API Key、令牌、私钥不得进入日志、截图或仓库。
- **文件与数据位置**：隐藏的 `~/.xjtu-launcher` 固定放设置、凭据与缓存（`config.json`、`auth.json`、`storage.json` 在顶层，`cache/`、`plugins/`（含 `plugins/settings/`）、`agent/`、`resources/`、`state/` 在子文件夹，`resources/icons/` 放用户自备的入口图标，见 [ICONS.md](dev-docs/reference/ICONS.md)）；用户自己的文件放可见的 `xjtu-launcher-file`（默认 `~/Documents/xjtu-launcher-file`，下有 `workspace/`、`plugin/` 与 `download/`），位置可在「应用设置 → 存储」改，记录在 `~/.xjtu-launcher/storage.json`。随应用发布的内容资源放仓库 `resources/`，界面运行时读取，不在代码里硬编码；`~/.xjtu-launcher/resources/community/` 可覆盖内置资源站点。界面路径一律显示为 `~/...`，不出现用户账号名。迁移必须先复制核对、再切换、最后删旧；`auth.json`、`config.json`、`storage.json` 与隐藏目录里的应用数据不参与迁移。
- **账号隔离**：切换 / 删除账号、重新认证时关闭旧校园窗口、丢弃旧请求，保留账号代次检查；缓存按账号隔离。
- **写操作**：评教、作业提交与工作台文件编辑都必须"先预览、用户明确要求、再 `confirm=true`"。结果不明不自动重试；取消不宣称撤回。工作台路径不得越出当前账号的工作区根目录；提交成功后的自动归档只复制并绑定，失败只警告。
- **插件与更新**：插件中心只展示随应用发布的内置插件，分校园广场 / 拓展发明 / 创新小港 / 已安装插件，没有远端市场、下载、安装与卸载。插件只有启用开关，侧栏入口跟随启用状态；插件设置存 `~/.xjtu-launcher/plugins/settings/`。不要写入假域名、假版本或伪造插件。
- **P0**：更新源暂用 HTTP `http://8.152.100.169/latest.json`（经下载网关读私有 OSS）；拿到域名与证书后必须切回 HTTPS，并恢复 `src-tauri/src/core/config.rs`、`src/features/settings/AppUpdateSettings.vue` 与 `tools/prepare-release.mjs` 的 HTTPS-only 校验，见 [P0.md](dev-docs/operations/P0.md)。
- **分支与推送**：分支命名不做要求；每个功能或修复完成后必须并入当前版本分支；不要凭记忆假设分支名，先看 `git branch -a`。发版时冻结版本分支，同步到 `dev` 做集成测试；`dev` 只做集成与测试，测试人员验证后给 PR 加 `tested` 标签，再由 `dev` 合入 `main`。`main` 是发布分支，禁止直接推送、删除与强推。`main` 更新后由工作流自动发版；每位开发者使用自己的 Git 身份（仓库级配置），不要冒用他人姓名或邮箱。
- **版本记录**：每次发版都按 [模板](docs/releases/TEMPLATE.md) 新建 `docs/releases/v<版本>.md` 并登记到 [索引](docs/releases/README.md)；GitHub Release 说明、镜像 `latest.json` 的 `notes` 与 README 下载链接必须指向同一版本。
- **验收表述**：区分"代码实现 / 自动测试通过 / 真实账号或真机验证 / 用户验收"。未部署的服务、未测试的平台、未获准的设计必须如实说明。开发与验收使用真正的 Tauri App。
- **文档分工**：`docs/` 面向使用者与产品（用户文档、设计稿、发布说明、截图与宣传图）；`dev-docs/` 面向开发者（约定、实现、发布流程、需求与归档）。根目录 `dev.md` 是开发入口，`todo_list.md` 只记没做的开发项，`acceptance_list.md` 是验收清单。规则只改 `dev-docs/CONVENTIONS.md`；开发进度只记在 `todo_list.md`（仅没做的开发项与未来方向），验收状态记在 `acceptance_list.md`，背景与实现细节在 `dev-docs/planning/need-do.md`；最后同步本文件的摘要。
