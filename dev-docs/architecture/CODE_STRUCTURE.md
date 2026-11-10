# 代码结构

本文记录当前代码的模块边界、新增代码应放的位置，以及后续可以继续拆分的部分。目标是让「页面 / 命令 / 领域服务 / 共享数据」各自有明确归属，而不是继续把所有逻辑堆进 `App.vue` 与 `lib.rs`。

## 一、Rust 后端

### 入口与共享状态

| 位置                      | 职责                                                                  |
| ------------------------- | --------------------------------------------------------------------- |
| `src-tauri/src/lib.rs`    | 模块声明、`run()`、Tauri 插件与窗口事件、`generate_handler!` 注册表   |
| `src-tauri/src/state.rs`  | `AppState`、锁、账号代次，以及 `main_only` / `config_path` 等共享助手 |
| `src-tauri/src/commands/` | Tauri 命令层，按业务域分文件；只做参数校验、加锁和调用领域模块        |

### 领域模块分组

单个 `.rs` 文件在 Rust 中就是一个模块，不是“没有模块化”。只有当一组模块需要共享命名空间，或一个模块要拆成多个子模块时，才建立文件夹。当前物理布局如下：

| 目录                       | 内容                                                                 |
| -------------------------- | -------------------------------------------------------------------- |
| `core/`                    | `config`、`storage`、`cache`、`network`、`vault`：路径、持久化与传输 |
| `identity/`                | `accounts`、`auth`：账号文件、登录与校园会话                         |
| `campus/`                  | `campus`（课表与校园协议）、`browser`、`timetable_custom`、`tool_names` |
| `school/`                  | 校园业务服务：查询、LMS、评教、图书馆、场馆等                        |
| `agent/`                   | Agent 运行时，以及 `providers`、`mcp`、`expose` 三个模型接入模块     |
| `plugins/`                 | `manifest`（内置插件清单）、`settings`（插件设置）、`capture`（回放截取） |
| `features/`                | `community`、`fonts`、`icons`、`personal_image`、`pet_desktop`、`updates` |
| `workbench/`               | 工作台目录、文件绑定与提交                                           |

`lib.rs` 把这批模块以 `pub(crate) use` 重新导出为 `crate::config`、`crate::auth` 等旧路径，命令层和其他模块无需跟着改调用点。新代码可以直接写真实路径（如 `crate::core::config`），也可以沿用根路径；同一个模块不要在两处各写一份逻辑。

`lib.rs` 不再承载业务命令。新增命令时：

1. 在 `commands/<domain>.rs` 增加 `#[tauri::command] pub(crate) fn ...`。
2. 在 `lib.rs` 的 `generate_handler!` 中用完整路径注册，例如 `commands::accounts::account_status`。
3. 需要给其他 Rust 模块复用的函数，在 `commands/mod.rs` 显式 `pub(crate) use`，不要用通配符把整个域暴露出去。
4. 只在命令层使用的结构体、锁和状态放回对应文件；跨命令共享的状态放 `state.rs`。

### 命令域

| 模块                    | 命令数 | 内容                                                     |
| ----------------------- | -----: | -------------------------------------------------------- |
| `commands/accounts.rs`  |     14 | 登录流程、账号切换 / 删除、头像、退出、API Key、账号状态 |
| `commands/agent.rs`     |     21 | 对话、历史、工作区、角色 / Skill / 任务、记忆、工具列表  |
| `commands/config.rs`    |      5 | 配置读写、插件列表与插件设置                             |
| `commands/expose.rs`    |      5 | 本机 MCP 端点状态、启停、端口与探测                      |
| `commands/lms.rs`       |      3 | 思源学堂附件打开 / 保存、回放地址                        |
| `commands/mcp.rs`       |      5 | 外部 MCP Server 配置与探测                               |
| `commands/network.rs`   |      3 | 网络自检、公开通知、外部链接与校园窗口                   |
| `commands/personal.rs`  |      2 | 个人页图片读取与来源                                     |
| `commands/providers.rs` |     10 | 模型厂商、模型目录、Key、余额与连通性                    |
| `commands/school.rs`    |      5 | 校园查询、评教预览 / 提交 / 取消、LMS 作业               |
| `commands/session.rs`   |      5 | 直连 / 主会话 / LMS / 图书馆 / 场馆会话刷新              |
| `commands/storage.rs`   |      6 | 数据目录状态、迁移预览 / 执行、打开目录、显示路径        |
| `commands/system.rs`    |      2 | 导出写入、入口图标                                       |
| `commands/timetable.rs` |      2 | 课表查询、本地课表编辑                                   |
| `commands/workbench.rs` |     11 | 工作台目录、文件、导入、绑定、删除、提交、同步           |

命令层之下是上表的领域模块。依赖方向是：

```text
commands/*  →  领域模块（agent / campus / school / workbench / identity / …）
            →  基础设施（core::config / core::storage / core::vault / core::cache）
```

领域模块之间需要复用的函数，继续通过 `crate::` 暴露或由 `commands/mod.rs` 显式转出；不要在模块里复制一份账号、会话或路径逻辑。

### 数据与安全边界

- 凭据、Cookie、API Key 只在 Rust 侧使用，命令返回值不得包含明文。
- 写操作统一在命令层校验 `confirm=true`；结果不明时不自动重试。
- `main_only` 必须保留在主窗口命令的入口；桌面宠物等独立窗口只注册自己的命令。

## 二、前端

### 页面与功能模块

`src/features/<domain>/` 是页面、页面私有组件和该功能逻辑的归属地。页面之间不直接互相引用；共用能力通过根目录服务或 `src/types/` 通信。

| 目录                   | 内容                                                                 |
| ---------------------- | -------------------------------------------------------------------- |
| `features/accounts/`   | 账号页、校园登录表单                                                 |
| `features/agent/`      | Agent 对话、历史、角色 / Skill / 任务、模型厂商、MCP 设置与暴露设置  |
| `features/appearance/` | 主页主题、字体美化、桌面宠物页，以及 `theme.ts` / `fonts.ts`         |
| `features/campus/`     | 畅行校园、课表、`components/` 下的校园子页面，以及登录 / 会话 / 预取 |
| `features/community/`  | 社区广场页面与搜索逻辑                                               |
| `features/home/`       | 主页与入口链接页                                                     |
| `features/personal/`   | 个人页、可编辑文本、图片裁剪                                         |
| `features/plugins/`    | 插件中心、回放截取页、`capture/` 截取流水线与 `pluginGlyph.ts`       |
| `features/settings/`   | 应用设置、软件更新设置、存储迁移设置                                 |
| `features/workbench/`  | 工作台页面                                                           |

### 共享层

| 位置                                                        | 职责                                                              |
| ----------------------------------------------------------- | ----------------------------------------------------------------- |
| `src/App.vue` / `src/main.ts` / `src/PetWindow.vue`         | 主窗口、桌宠窗口入口与应用外壳                                    |
| `src/components/ui/`                                        | 通用控件：按钮、标签页、设置列表、弹窗、空状态等                  |
| `src/components/*.vue`                                      | 仍被多个功能共用的组件，如 `SidebarPet`、`PetArt`、`UpdateDialog` |
| `src/types/<domain>.ts`                                     | IPC 与页面的领域类型；`src/types/index.ts` 统一转出               |
| `src/shared/`                                               | 跨功能逻辑：`bridge`、`cache`、`domain`、`icons`、`markdown`、`notices`、`pets`、`updates` |
| `src/composables/`                                          | 跨页面组合式函数：侧栏拖拽、滑动指示器                            |
| `src/styles/`                                               | 全局 token、控件、页面、壁纸与桌宠样式                            |
| `src/dev/`                                                  | 仅开发期使用的图标对比页，不进入产品导航                          |

### 新增前端代码

1. 新页面放 `src/features/<domain>/`，页面私有组件放同目录；只有真正跨功能复用才提升到 `src/components/` 或 `src/components/ui/`。
2. 新类型放 `src/types/<domain>.ts`，并在 `src/types/index.ts` 增加 `export *`；不要在页面里定义另一套同名 IPC 类型。
3. 页面只通过 `shared/bridge.ts` 调用后端；不要在组件里直接 `invoke`。
4. 可复用状态逻辑写成 `features/<domain>/` 内的 `.ts`，页面保持展示与交互；需要跨页面复用的逻辑才放 `src/shared/`，组合式函数放 `src/composables/`。
5. 新模块的测试与实现同目录，例如 `features/campus/campusSession.test.ts`。

## 三、当前重构状态与后续

已经完成（2026-11-11）：

- `lib.rs` 从约 3300 行拆为入口 + `state.rs` + 15 个命令域文件，命令仍逐个注册。
- 后端平铺模块按 `core/`、`identity/`、`campus/`、`agent/`、`plugins/`、`features/`、`workbench/` 收拢；`plugins/manifest` 仍可被 smoke 示例单独编译。
- 前端页面从扁平 `src/components/` 迁入 `src/features/`，`src/types.ts` 拆为 14 个领域文件并由 `src/types/index.ts` 转出；跨功能逻辑、组合式函数、样式和开发页分别收进 `src/shared/`、`src/composables/`、`src/styles/`、`src/dev/`。
- 组件移动后 `npm run build`、前端测试、Rust 测试、Clippy 与格式检查均通过。

仍可继续拆分，但不应为了拆而拆：

- `App.vue` 仍承担应用外壳与路由；后续可把侧栏导航、窗口事件、更新对话框分别抽成 composable。
- `AgentView.vue`、`PptCaptureView.vue`、`WorkbenchView.vue` 仍较大，可按「列表 / 详情 / 提交预览」继续拆私有组件。
- `commands/school.rs` 同时包含查询、评教与 LMS 作业，后续若继续膨胀，可再拆 `commands/school_query.rs`、`commands/evaluations.rs`、`commands/lms_homework.rs`。
- 根目录的 `bridge.ts`、`cache.ts`、`domain.ts` 仍服务多个功能；等出现第二个明确使用者时再迁入 `src/core/`，避免提前分层。

任何拆分都必须保持命令名、IPC 参数、凭证边界和写操作确认流程不变；重构后按改动范围重新跑构建、测试、Clippy 与格式检查。
