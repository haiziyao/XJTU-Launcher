# 桌面应用更新发布

客户端已接入 Tauri Updater：检查新版、说明、下载进度、签名验证、安装及重启。默认开发构建未配置更新端点和公钥，显示“此构建尚未配置更新服务”；不会将其误报为最新版本。

插件市场地址与应用更新端点分别配置。修改插件来源不会修改应用更新来源。

## 首次部署

1. 使用 Tauri signer 生成发布密钥；私钥只存入发布系统的 Secret，公钥供客户端编译使用。
2. 在仓库 Variables 中设置 `XJTU_UPDATE_ENDPOINT`（HTTPS 的静态更新 JSON 地址）和 `XJTU_UPDATE_PUBLIC_KEY`。可选设置 `XJTU_PLUGIN_SERVER`。
3. 在 Secrets 中设置 `TAURI_SIGNING_PRIVATE_KEY` 和相应的 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。不把私钥写入仓库。
4. 保持 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 的版本一致。
5. 手动运行 `Signed desktop release` 工作流，构建 macOS Apple Silicon、macOS Intel 和 Windows x64，生成带签名的更新包、更新 JSON 与草稿 Release。检查安装包和更新清单后再正式发布；公钥配置必须已包含在用户当前安装的客户端中。

工作流调用 `tools/prepare-release.mjs`，生成开启 `bundle.createUpdaterArtifacts` 的临时 Tauri 配置。普通 `npm run app:build` 保持可本地构建，无需发布密钥。更新相关环境变量也通过 Rust 编译进入客户端，不允许用户更改插件源时连带修改。

GitHub Releases 公共仓库可将更新端点配置为发布资产中的 latest.json；若仓库私有，应使用用户无需 GitHub 凭据即可访问的自有公开更新托管。更新清单中的平台名称、下载 URL 和 `.sig` 内容由官方 Tauri Action 生成。当前没有部署正式端点，也没有创建或使用发布私钥。

## 运行行为

启动后按用户选项自动检查；手动检查位于“应用设置 → 软件更新”。下载与状态保存在 Rust 运行时，切换页面不会中断。安装与重启前尝试取得校园任务、账号操作和插件安装锁，已有操作未结束则提示稍后重试。macOS 安装完成后由用户选择重启，Windows 由安装程序按 Updater 机制退出并更新。

未来 Agent 有实际运行任务后，必须把任务锁接入更新前检查，不能宣称目前已经覆盖尚不存在的 Agent 运行时。

参考：Tauri Updater 官方文档与 `tauri-apps/tauri-action` 官方 README。未使用真实发布包进行跨版本安装，Windows 安装行为仍需 Windows 实机验收。
