# 插件中心与软件更新

更新时间：2026-11-08。插件部分已被当天用户决定取代：**不再有远端插件市场**，插件中心只展示随应用发布的内置插件。软件更新部分继续有效。

## 当前实现（2026-11-08）

插件中心分「校园广场」「拓展发明」「创新小港」「已安装插件」四个分区，只保留启用开关；没有下载、安装、卸载、本地导入、服务端地址或插件目录。课表在校园广场并默认启用；桌面宠物与字体美化在拓展发明并默认关闭；回放截取在创新小港。行式布局由用户当次确认，插件清单格式、分类与设置存储见 [插件格式与插件中心](PLUGINS.md)。

已删除：`src-tauri/src/market.rs`、前端 `plugin_catalog` / `plugin_source` / `import_plugin` / `install_market_plugin` / `remove_plugin`、插件服务设置页、`tools/plugin-server/`、`examples/campus-links/`、`market_smoke` 示例、MCP 远端目录（`mcp_catalog` / `install_mcp_entry`）及 `market_lock`。MCP 本地手动添加仍保留。

旧配置的 `pluginServer`、`checkPluginUpdates` 与桌面宠物字段仍能读取，用于把桌面宠物设置迁移到 `plugins/settings/`，之后不再写入；`pluginLayout` 记录这次布局迁移。

应用更新未受本轮影响，见下节。

## 软件更新

选用 Tauri 2 官方 Updater 插件。2026-10-19 已核对官方文档源文件：

```text
https://v2.tauri.app/plugin/updater/
https://raw.githubusercontent.com/tauri-apps/tauri-docs/v2/src/content/docs/plugin/updater.mdx
```

核实事项：

- 支持动态更新服务端或静态 JSON 更新清单；官方 Tauri Action 可以生成用于 GitHub Releases 的更新清单。
- `createUpdaterArtifacts: true` 生成更新产物。macOS 使用 App 的 tar.gz 归档；Windows 使用 MSI/NSIS 安装包及各自签名。
- 更新插件校验发布签名，此校验不能关闭。客户端内置公钥，发布流程使用私钥；这是软件更新校验，与校园密码的本地保存方案独立。
- 静态清单按操作系统和架构分别提供下载地址与签名，并包含版本号及可选更新说明。

应用已引入官方 Updater，支持检查、下载、安装、重启和独立的自动检查开关；构建时配置更新端点和公钥。未配置时明确显示未配置。发布工作流已跑通，0.5.2 起带签名更新发布；更新镜像部署在自有服务器上，暂时只能用 HTTP 直连 IP（P0，见 [P0.md](P0.md)）。细节见 [发布指南](RELEASING.md)。

安装前仍检查正在执行的校园与账号任务；插件安装锁已随远端市场删除。

## 插件运行时与版本边界

2026-10-29 更新：**已决定不引入通用插件运行时**。插件由项目自己实现并随 App 版本发布，WASM / sidecar 两条路线暂不选型。2026-11-08 起连远端下载也不再提供，插件清单只允许内置 id。下载与卸载是否可行尚未评估，本轮不做。

后续若要重新开放第三方插件，需要先明确包清单、兼容版本、来源与安装目录，并单独设计签名、下载暂存、校验、原子安装和失败回滚；下载清单不能冒充已经支持运行任意远端代码。

## 历史设计

`docs/design/plugin-market-v2/` 的插件市场参考图与三分区实现是 2026-10-19 的历史基线，已被本轮取代；其中软件更新页继续有效。Agent 与评教旧稿仍处于“需要调整”状态，不因插件改动自动通过。

## 本轮验证

- Rust：`cargo test` 161 项通过（14 项真实网络 / 原生窗口测试默认忽略），`cargo clippy --all-targets` 零警告。
- 前端：`vue-tsc --noEmit` 通过，Vitest 82 项通过，Prettier 检查通过。
- 尚未构建验收包、未在真机逐项点击插件中心与字体/桌宠页面，未做 Windows 实机验收；未部署正式更新域名。
