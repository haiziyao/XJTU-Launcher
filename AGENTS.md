# 项目工作约定

本文件给 coding agent 提供**必须遵守的硬约束**。完整规则见 [约定手册](dev-docs/CONVENTIONS.md)，本文件不重复细节；两者冲突时以约定手册为准。

开始工作先读：[开发文档索引](dev-docs/README.md)、[开发指南](dev-docs/DEVELOPMENT.md)、[约定手册](dev-docs/CONVENTIONS.md)、[需求时间线](dev-docs/need-do.md)。历史讨论在 `dev-docs/archive/`，不能用旧记录覆盖后来明确的用户决定。

## 硬约束

- **设计**：新前端必须先有参考图，且用户明确同意设计与开始实现。`docs/design/plugin-market-v2/` 已获准；`campus-agent-v1` 的 Agent / 评教页与最小化桌宠设计**未获准**，不得按旧稿或设想直接改视觉。
- **技术栈**：Rust + Tauri 2 + Vue 3 + TypeScript，目标 macOS / Windows。校园功能先研究本地 XJTUToolBox，再独立适配；复用 GPL-3.0 代码前核对分发要求。
- **凭据**：保存在 `~/.xjtu-launcher/auth.json`，不使用系统钥匙串。密码、Cookie、API Key、令牌、私钥不得进入日志、截图或仓库。
- **账号隔离**：切换 / 删除账号、重新认证时关闭旧校园窗口、丢弃旧请求，保留账号代次检查；缓存按账号隔离。
- **写操作**：评教与作业提交都必须"先预览、用户明确要求、再 `confirm=true`"。结果不明不自动重试；取消不宣称撤回。
- **插件与更新**：插件只有启用开关，侧栏入口跟随启用状态；服务端地址默认只读，确认危险提示后才可编辑。远端包只支持声明式导航。不要写入假域名、假版本或伪造插件。
- **P0**：更新镜像暂用 HTTP `http://8.152.100.169/xjtu-launcher/latest.json`；拿到域名与证书后必须切回 HTTPS，并恢复 `config.rs`、`AppUpdateSettings.vue`、`prepare-release.mjs` 的 HTTPS-only 校验，见 [P0.md](dev-docs/P0.md)。
- **分支与推送**：`main` 为默认分支，日常在 `codex/<主题>` 分支；推送、打标签、发布 Release 只按用户当次明确要求执行，不擅自扩大范围。提交身份用仓库级 `haiziyao <1754203003@qq.com>`。
- **验收表述**：区分"代码实现 / 自动测试通过 / 真实账号或真机验证 / 用户验收"。未部署的服务、未测试的平台、未获准的设计必须如实说明。开发与验收使用真正的 Tauri App。
- **文档分工**：`docs/` 面向使用者与产品（用户文档、设计稿、发布说明）；`dev-docs/` 面向开发者（约定、实现、发布流程、需求与归档）。规则只改 `dev-docs/CONVENTIONS.md`，需求只改 `dev-docs/need-do.md`，再同步本文件的摘要。
