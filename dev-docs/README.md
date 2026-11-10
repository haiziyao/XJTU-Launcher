# 开发文档

本目录是工程内部资料。面向使用者的文档在 `docs/`，根目录 [dev.md](../dev.md) 是开发第一入口。

分支流程（2026-11-11 起）：分支命名不做要求；每个功能或修复完成后并入当前版本分支；发版时冻结版本分支，同步到 `dev` 做集成测试；测试通过后由 `dev` 创建 PR 合入 `main`，`main` 更新后自动构建发布。版本分支名查看 `git branch -a`，不要凭记忆推断。

## 目录分工

| 目录                     | 面向           | 内容                                                    |
| ------------------------ | -------------- | ------------------------------------------------------- |
| `docs/`                  | 使用者与产品   | 根目录 README、设计稿与评审、版本发布说明、截图与宣传图 |
| `dev-docs/`              | 开发者         | 入口、规则、开发指南；子目录按主题分类                  |
| `dev-docs/architecture/` | 开发者         | 代码模块边界、目录职责与重构顺序                        |
| `dev-docs/features/`     | 开发者         | 插件、Agent、校园功能与回放截取等具体能力               |
| `dev-docs/reference/`    | 开发者         | 组件与图标规范、参考项目研究                            |
| `dev-docs/operations/`   | 发布与运维     | 发版、签名、官网、P0 与本机基础设施                     |
| `dev-docs/planning/`     | 产品与开发     | 需求时间线、依赖与实现背景                              |
| `dev-docs/archive/`      | 追溯           | 历史讨论、旧版实现记录、旧约定原文；不作为当前规则      |
| `skills/`                | 开发者与 Agent | 项目开发 skill：样式、内置插件、推送与发版              |

判断标准：读者是**使用者或产品决策者**就放 `docs/`；读者是**写代码、发布或验收的人**就放 `dev-docs/`。两边都需要的结论（如“当前版本能做什么”）在 `docs/` 面向用户表述，在 `dev-docs/` 记录实现与验收边界。

## 从这里开始

1. [开发须知](../dev.md)：环境、目录、分支、检查命令与数据安全速查。
2. [开发待办清单](../todo_list.md)：只记没做的开发项与未来方向；验收状态在 [验收清单](../acceptance_list.md)；详细背景在 [需求时间线](planning/need-do.md)。
3. [约定手册](CONVENTIONS.md)：**唯一规则来源**。设计授权、数据安全、分支发布、验收标准都在这里。
4. [开发指南](DEVELOPMENT.md)：启动方式、检查命令、当前实现与接续任务。
5. [代码结构](architecture/CODE_STRUCTURE.md)：Rust / Vue 模块边界与重构顺序。
6. [需求时间线](planning/need-do.md)：按状态与优先级排列的待做需求、依赖和验收条件。
7. [P0 待办](operations/P0.md)：正式发布前必须补齐的安全与可用性缺口。
8. [根目录 AGENTS.md](../AGENTS.md)：给 coding agent 的硬约束摘要。

## 按主题

### 架构与开发

| 文档                                         | 内容                                     |
| -------------------------------------------- | ---------------------------------------- |
| [开发指南](DEVELOPMENT.md)                   | 环境、启动、验证、当前可用与未完成项     |
| [代码结构](architecture/CODE_STRUCTURE.md)   | Rust 命令层、领域模块与 Vue 功能目录     |
| [按钮与通用控件](reference/UI_COMPONENTS.md) | `UiButton` / `UiIconButton` 的变体与用法 |
| [自定义入口图标](reference/ICONS.md)         | 用户自备图标目录、`icons.json` 映射规则  |

### 功能实现

| 文档                                                    | 内容                                                     |
| ------------------------------------------------------- | -------------------------------------------------------- |
| [插件格式](features/PLUGINS.md)                         | 内置插件清单、插件中心与设置存储                         |
| [插件中心与更新](features/PLUGIN_MARKET_AND_UPDATES.md) | 内置插件中心、软件更新与部署边界                         |
| [回放截取插件方案](features/PLUGIN_PPT_CAPTURE.md)      | 内置插件、截取流水线与分阶段实施（已实现，真机验收待做） |
| [校园功能接续](features/CAMPUS_FEATURES.md)             | 适配清单、评教后台与写能力边界                           |
| [Agent 与 Agent 设置](features/AGENT.md)                | 当前结构、工具边界、数据与安全                           |

### 发布与运维

| 文档                                           | 内容                                                      |
| ---------------------------------------------- | --------------------------------------------------------- |
| [发布指南](operations/RELEASING.md)            | 签名更新、镜像更新源、发布变量与草稿工作流                |
| [官网与下载分发](operations/WEBSITE.md)        | 官网源码、服务器部署、OSS 安装包与 HTTPS 迁移             |
| [OSS 下载网关](../tools/oss-gateway/README.md) | 私有 Bucket、60 秒签名链接、Cookie 下载配额与 IP 兜底限流 |
| [版本记录](../docs/releases/README.md)         | 每个版本的改动、验证与撤回情况                            |
| [安装包签名](operations/SIGNING.md)            | macOS Developer ID 与公证、Windows 代码签名进度           |
| [P0 待办](operations/P0.md)                    | 发布前必须处理的事项（当前：更新镜像使用 HTTP）           |
| [本机基础设施](operations/local/infra.md)      | 服务器、OSS、网关与更新源的本机记录                       |

### 参考与归档

- [参考项目](reference/REFERENCE_PROJECT.md)：XJTUToolBox 研究入口和许可边界。
- `archive/`：项目上下文、V0.1 记录、旧约定原文、已完成的 v0.6 组件交接，以及 2026-10-23 的 Agent 设计记录。只用于追溯，最新状态以本目录文档为准。
- 设计与设计评审在 [`docs/design/`](../docs/design/)，属于产品资料，不在本目录。

## 项目 skill

根目录 [skills/](../skills/) 下的 skill 由 Claude Code 按名称加载，也可以手动阅读：

| skill          | 用途                                                  |
| -------------- | ----------------------------------------------------- |
| `xjtul-style`  | 前端视觉、样式、组件选择、壁纸与验收清单              |
| `xjtul-plugin` | 新增 / 修改内置插件：清单、注册、页面路由、设置与安全 |
| `xjtul-push`   | 提交、推送、合并分支、版本号、Release 与 OSS          |

## 更新规则

- 规则变化先改 [CONVENTIONS.md](CONVENTIONS.md)，再同步 `AGENTS.md` 的硬约束摘要；不要在两处各写一份完整规则。
- 开发进度只改 [todo_list.md](../todo_list.md)（仅没做的开发项与未来方向）；验收状态改 [acceptance_list.md](../acceptance_list.md)；背景、依赖与实现细节在 [need-do.md](planning/need-do.md)。完成的条目写日期与版本。
- 实现状态变化同步 [DEVELOPMENT.md](DEVELOPMENT.md) 的「当前可用与未完成项」。
- 移动或重命名本文档时，同步更新根目录 README、`dev.md`、`AGENTS.md` 和所有交叉链接。

更新日期：2026-11-11。
