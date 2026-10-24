# 桌面应用更新发布

客户端已接入 Tauri Updater：检查新版、说明、下载进度、签名验证、安装及重启。默认开发构建未配置更新端点和公钥，显示“此构建尚未配置更新服务”；不会将其误报为最新版本。

插件市场地址与应用更新端点分别配置。修改插件来源不会修改应用更新来源。

## 当前配置（2026-10-22）

- 更新端点：`https://github.com/haiziyao/XJTU-Launcher/releases/latest/download/latest.json`（仓库为公开仓库，使用 Release 资产中的 latest.json）。
- 更新公钥：`dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDZCRDUwMjM1QzIzRTVBRkIKUldUN1dqN0NOUUxWYTQ0WlRpUGZWeVg1V21OVDZna1BsdW9kSExSWitYaUNHay9TL2JZU0hkSnIK`
- 签名密钥已在本机生成（`~/.tauri/xjtu-launcher.key`，无密码）；私钥不入仓库，需自行备份并填入 GitHub Secret。丢失私钥后无法再为已发布客户端签名更新。
- 仓库 Variables 与 Secret 已于 2026-10-22 配置完毕，签名构建可跑通。

系统代码签名（Gatekeeper / SmartScreen）尚未配置，步骤见 [SIGNING.md](SIGNING.md)。未签名不影响自动更新的 minisign 校验，但首次安装会触发系统警告。

## 首次部署

1. 使用 Tauri signer 生成发布密钥；私钥只存入发布系统的 Secret，公钥供客户端编译使用。
2. 在仓库 Variables 中设置 `XJTU_UPDATE_ENDPOINT`（HTTPS 的静态更新 JSON 地址）和 `XJTU_UPDATE_PUBLIC_KEY`。可选设置 `XJTU_PLUGIN_SERVER`。
3. 在 Secrets 中设置 `TAURI_SIGNING_PRIVATE_KEY` 和相应的 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。不把私钥写入仓库。
4. 保持 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 的版本一致。
5. 手动运行 `Signed desktop release` 工作流，构建 macOS Apple Silicon、macOS Intel 和 Windows x64，生成带签名的更新包、更新 JSON 与草稿 Release。检查安装包和更新清单后再正式发布；公钥配置必须已包含在用户当前安装的客户端中。

工作流把三个平台串行执行（`max-parallel: 1`）。`tauri-action` 无法按 tag 查询草稿，只能取发布列表里第一个同名草稿，并行 job 会各自新建草稿并把资产分散上传，导致每个草稿都缺平台、`latest.json` 只有部分 `platforms`。发布前确认**只有一个**同名草稿且三个平台的安装包、更新包和 `.sig` 齐全，否则先删除残缺草稿再重跑。每次构建都会先读出已有 `latest.json` 再合并平台，挂到同一个草稿上即可得到完整清单。

工作流调用 `tools/prepare-release.mjs`，生成开启 `bundle.createUpdaterArtifacts` 的临时 Tauri 配置。普通 `npm run app:build` 保持可本地构建，无需发布密钥。更新相关环境变量也通过 Rust 编译进入客户端，不允许用户更改插件源时连带修改。

GitHub Releases 公共仓库可将更新端点配置为发布资产中的 latest.json；若仓库私有，应使用用户无需 GitHub 凭据即可访问的自有公开更新托管。更新清单中的平台名称、下载 URL 和 `.sig` 内容由官方 Tauri Action 生成。清单里的下载 URL 是 `api.github.com/.../releases/assets/{id}` 形式，客户端下载时会带 `Accept: application/octet-stream`，公开仓库正式发布后可匿名下载。签名构建已于 2026-10-22 跑通，但尚无正式发布的版本，跨版本更新仍未实测。

## 无法访问 GitHub 的用户 / 镜像更新源

应用更新默认读取 GitHub Releases 的 `latest.json`。网络无法访问 GitHub 时，用户可以在「应用设置 → 软件更新 → 更新源」填写镜像地址（必须是 HTTPS，不能含账号密码）；留空则使用内置地址。

镜像需要同时提供：

1. `latest.json`，其中的 `platforms[*].url` 必须指向镜像自己的安装包，而不是 GitHub 的 `api.github.com/.../assets/...`；
2. 各平台安装包 / 更新包与 `.sig`，内容与签名必须和原发布一致。

只有镜像 `latest.json` 而下载地址仍指向 GitHub 时，检查更新能通过但下载仍会被网络挡住。镜像部署完成后，在客户端填入 `https://<镜像>/latest.json` 并点击「检查更新」即可验证。

## 发布后

- 更新 README「下载安装」里的三条直达链接到新版本的资产名（`vX.Y.Z` 与文件名都含版本号）；否则首页按钮会指向旧版本。
- 在 GitHub Releases 页面确认新版本已标记为 Latest。

## 运行行为

启动后按用户选项自动检查；手动检查位于“应用设置 → 软件更新”。下载与状态保存在 Rust 运行时，切换页面不会中断。安装与重启前尝试取得校园任务、账号操作和插件安装锁，已有操作未结束则提示稍后重试。macOS 安装完成后由用户选择重启，Windows 由安装程序按 Updater 机制退出并更新。

未来 Agent 有实际运行任务后，必须把任务锁接入更新前检查，不能宣称目前已经覆盖尚不存在的 Agent 运行时。

参考：Tauri Updater 官方文档与 `tauri-apps/tauri-action` 官方 README。未使用真实发布包进行跨版本安装，Windows 安装行为仍需 Windows 实机验收。
