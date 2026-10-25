# 开发文档

此目录的开发资料随代码一起维护，`main` 上的内容与最新发布版一致；日常开发在 `codex/*` 分支进行，完成后合入 `main`。

## 从这里开始

1. [开发指南](DEVELOPMENT.md)：目录、启动、验证、当前实现与接续任务。
2. [约定手册](CONVENTIONS.md)：设计确认、分支管理、组件复用、数据和验收规则。
3. [根目录 AGENTS.md](../AGENTS.md)：供后续开发者与 coding agent 读取的有效规则。
4. [P0 待办](P0.md)：正式发布前必须处理的安全与可用性缺口（当前：更新镜像临时使用 HTTP）。

## 功能与协议

| 文档                                           | 内容                                            |
| ---------------------------------------------- | ----------------------------------------------- |
| [按钮与通用控件](UI_COMPONENTS.md)             | `UiButton` / `UiIconButton` 的样式、变体和用法  |
| [插件格式](PLUGINS.md)                         | 声明式插件、安装和管理                          |
| [插件市场与更新](PLUGIN_MARKET_AND_UPDATES.md) | 已获准方案、实现及部署边界                      |
| [插件服务](../tools/plugin-server/README.md)   | 本地服务、目录契约和真实 HTTP 联调              |
| [发布指南](RELEASING.md)                       | 签名更新、镜像更新源、发布变量与草稿工作流      |
| [安装包签名](SIGNING.md)                       | macOS Developer ID 与公证、Windows 代码签名进度 |
| [校园功能接续](CAMPUS_FEATURES.md)             | 适配清单、评教后台与写能力边界                  |
| [Agent 与 Agent 设置](AGENT.md)                | 2026-10-23 最新决定、当前实现状态与实现顺序     |
| [参考项目](REFERENCE_PROJECT.md)               | XJTUToolBox 研究入口和许可边界                  |

## 设计与历史

- [插件市场 v2](design/plugin-market-v2/README.md)：已确认，已实施。
- [校园功能与 Agent v1](design/campus-agent-v1/README.md)：旧稿需调整，Agent / 评教页面未获准；2026-10-23 改为 Agent 设置插件方案，见 [Agent 与 Agent 设置](AGENT.md)。
- [桌面参考](design/DESKTOP_REFERENCES.md)、[早期页面评审](design/PAGES_REVIEW.md)：历史设计依据，不能视为新的实施授权。
- [项目上下文](PROJECT_CONTEXT.md)、[V0.1 记录](V0.1.md)：讨论与迭代记录；最新状态以开发指南和有效约定为准。
- `history/`：初始需求与旧版约定原文，保留追溯，不作为当前规则。

更新日期：2026-10-25。
