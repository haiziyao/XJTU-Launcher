# 开发须知

本文件是参与开发的第一入口。开始任何任务前，按顺序读：

1. 本文件：环境、目录、分支与提交、检查命令。
2. [todo_list.md](todo_list.md) 与 [acceptance_list.md](acceptance_list.md)：前者只记没做的开发项与未来方向，后者记录待验收与已验证内容。
3. [dev-docs/README.md](dev-docs/README.md)：开发文档索引。
4. [dev-docs/CONVENTIONS.md](dev-docs/CONVENTIONS.md)：**唯一规则来源**。本文件与它冲突时，以约定手册为准。

产品面向西安交通大学学生，技术栈固定为 **Rust + Tauri 2 + Vue 3 + TypeScript**，优先 macOS，其次 Windows。当前版本 **0.6.6**（2026-11-10 发布）。

## 环境

- Node.js 22.12+ 与 npm，依赖由 `package-lock.json` 固定。
- Rust 工具链，依赖由 `src-tauri/Cargo.lock` 固定。
- macOS：Xcode Command Line Tools。Windows：Visual Studio C++ Build Tools 与 WebView2。

```sh
npm ci
npm run dev
```

`npm run dev` 启动真正的 Tauri 桌面窗口；`npm run dev:frontend` 只跑网页资源，不能代替客户端验收。

macOS 构建独立 App：

```sh
npm run app:build -- --bundles app
open "src-tauri/target/release/bundle/macos/XJTU Launcher.app"
```

cargo 不在 PATH 时先执行 `source "$HOME/.cargo/env"`。普通本地构建不需要发布签名密钥。

## 目录

| 路径                      | 职责                                                     |
| ------------------------- | -------------------------------------------------------- |
| `src/`                    | Vue 入口（`App.vue`、`main.ts`、`PetWindow.vue`）与功能目录 |
| `src/features/`           | 按功能分的页面、页面私有组件与逻辑                       |
| `src/types/`              | 领域类型；`index.ts` 统一转出                            |
| `src/shared/`             | 跨功能逻辑：IPC 封装、缓存、领域计算、图标与通知         |
| `src/composables/`        | 跨页面组合式函数                                         |
| `src/styles/`             | 全局 token、控件、页面、壁纸与桌宠样式                   |
| `src/dev/`                | 仅开发期使用的图标对比页                                 |
| `src/components/ui/`      | 通用组件，新增界面优先复用                               |
| `src-tauri/src/`          | Rust 后端入口、状态与领域模块目录                        |
| `src-tauri/src/commands/` | Tauri 命令，按业务域分文件                               |
| `src-tauri/src/core/`     | 配置、存储、缓存、网络与凭据等基础设施                   |
| `src-tauri/src/campus/`   | 校园协议、课表、校园窗口与本地课表编辑                   |
| `src-tauri/src/school/`   | 校园业务服务：查询、LMS、评教、图书馆与场馆              |
| `src-tauri/src/agent/`    | Agent 运行时、模型厂商、MCP 与本机端点                   |
| `src-tauri/src/plugins/`  | 内置插件清单、插件设置与回放截取存储                     |
| `src-tauri/src/features/` | 社区、字体、图标、个人图片、桌宠与更新                   |
| `src-tauri/src/workbench/` | 工作台文件与提交服务                                    |
| `plugins/`                | 随应用发布的内置插件清单                                 |
| `tools/`                  | 发布、官网、OSS 下载网关脚本                             |
| `website/`                | `xjtu-launcher.cn` 静态官网，无构建步骤                  |
| `docs/`                   | 面向使用者与产品：README、设计稿、版本记录、截图与宣传图 |
| `dev-docs/`               | 面向开发者：约定、开发指南、架构、需求时间线、发布与签名 |
| `skills/`                 | 项目开发 skill，见下文                                   |

前端页面在 `src/features/`，样式 token 在 `src/styles/style.css`，共用控件在 `src/styles/ui.css`，页面样式在 `src/styles/pages.css`，桌面外壳与壁纸在 `src/styles/desktop-shell.css`。模块边界和新增代码的位置见 [代码结构](dev-docs/architecture/CODE_STRUCTURE.md)。

## 分支、合并与发版

2026-11-11 起采用多人协作流程。文档只写长期存在的 `main` 和 `dev`；**不要凭记忆假设版本分支名**，开始前先查看仓库当前状态：

```sh
git branch -a
```

- **分支命名不做要求**。可以按主题或 issue 自建分支。
- **每个功能或 bug 修完后，必须并入当前版本分支**。不要把完成的工作长期留在个人分支。
- 版本分支在开发期可以自由提交；到发版时间点**冻结**，不再接受新功能。
- 冻结后把版本分支同步到 `dev`，由测试人员做集成测试。`dev` 只做集成与测试，不直接开发。
- 测试通过后，由 `dev` 创建 PR 合入 `main`，测试人员给 PR 加 `tested` 标签；`main` 更新后自动构建发布。
- `main` 是发布分支，受分支规则保护：禁止直接推送、删除和强推，只接受 `dev` 来源的 PR。

```sh
git switch <当前版本分支>
git add -A
git commit -m "feat: ..."
git push -u origin <当前分支>
```

提交前确认使用的是**你自己的** Git 身份，不要冒用项目维护者或他人的姓名、邮箱：

```sh
git config --show-origin --get user.name
git config --show-origin --get user.email
```

未配置时用 `git config --local` 设置自己的信息；不要把真实姓名、邮箱写进共享文档或 skill。

推送、合并到版本分支可以随任务执行；合入 `dev` / `main`、发版和撤回 Release 仍按用户当次明确要求执行。

## 检查

提交前至少跑与改动相关的检查；较大一批修改统一执行：

```sh
npm test
npm run build
npm run format:check
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

只改前端时至少 `npm run build`、`npm test`、`npm run format:check`；只改 Rust 时至少 `cargo test`、`cargo fmt --check`、`cargo clippy -D warnings`。

## 设计与前端

- 新前端**先有参考图**，用户明确同意设计和开始实现后再动手。技术方案、功能需求或“先做个页面”的同意，都不等于设计获批。
- 复用 `src/components/ui/`、现有 token 和图标，不临时另造视觉系统。按钮只用 `UiButton` / `UiIconButton`，页面标签只用 `PageTabs` / `SegmentedTabs`。
- 设置页用列表 + 弹出卡片，不写解释性小字；新增设置项放列表末尾。
- 有壁纸时控件用 `--control-bg` / `--control-border` 等半透明 token，不要用不透明白块盖住壁纸。
- 状态不能只靠颜色，必须同时有图标或文字；尊重 `prefers-reduced-motion`。
- 视觉细节见 [dev-docs/reference/UI_COMPONENTS.md](dev-docs/reference/UI_COMPONENTS.md)，完整规则见约定手册第 2 节。

## 数据与安全

- 凭据保存在 `~/.xjtu-launcher/auth.json`，不使用系统钥匙串。密码、Cookie、API Key、令牌、私钥**不得**进入日志、截图或仓库。
- `~/.xjtu-launcher` 固定放设置、凭据与缓存；用户文件放可见的 `xjtu-launcher-file`（默认 `~/Documents/xjtu-launcher-file`，含 `workspace/`、`plugin/`、`download/`）。
- 界面路径一律显示为 `~/...`，不出现用户账号名。
- 切换 / 删除账号时关闭旧校园窗口、丢弃旧请求，保留账号代次检查；缓存按账号隔离。
- 评教、作业提交、工作台文件编辑必须“先预览、用户明确要求、再 `confirm=true`”。结果不明不自动重试，取消不宣称撤回。
- 不把校园凭据发送给任何插件或外部服务。

## 文档与待办

- `docs/` 面向使用者与产品；`dev-docs/` 面向开发者。判断标准是读者是谁。
- 规则变化先改 [dev-docs/CONVENTIONS.md](dev-docs/CONVENTIONS.md)，再同步 [AGENTS.md](AGENTS.md) 的硬约束摘要。
- 开发进度改 [todo_list.md](todo_list.md)；验收状态改 [acceptance_list.md](acceptance_list.md)；背景、依赖与实现记录写在 [dev-docs/planning/need-do.md](dev-docs/planning/need-do.md)。
- 实现状态变化同步 [dev-docs/DEVELOPMENT.md](dev-docs/DEVELOPMENT.md) 的“当前可用与未完成项”。
- 每次发版按 [模板](docs/releases/TEMPLATE.md) 新建版本记录并登记索引。
- 验收表述必须区分：**代码实现 / 自动测试通过 / 真实账号或真机验证 / 用户验收**。未部署的服务、未测试的平台、未获准的设计要如实说明。

## 开发 skill

项目 skill 在根目录 [skills/](skills/)，说明见 [skills/README.md](skills/README.md)：

| skill          | 用途                                                    |
| -------------- | ------------------------------------------------------- |
| `xjtul-style`  | 改前端视觉、样式、组件前读；包含 token、组件与壁纸规则  |
| `xjtul-plugin` | 新增或修改内置插件；清单、页面路由、设置与安全边界      |
| `xjtul-push`   | 提交、推送、合并分支与发版；版本号、Release 与 OSS 流程 |

直接说“按 xjtul-style 做”即可让 Claude Code 使用对应 skill。
