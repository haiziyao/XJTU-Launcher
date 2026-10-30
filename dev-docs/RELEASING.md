# 桌面应用更新发布

客户端已接入 Tauri Updater：检查新版、说明、下载进度、签名验证、安装及重启。默认开发构建未配置更新端点和公钥，显示“此构建尚未配置更新服务”；不会将其误报为最新版本。

插件市场地址与应用更新端点分别配置。修改插件来源不会修改应用更新来源。

## 当前配置（2026-10-22）

- 更新端点：`https://github.com/haiziyao/XJTU-Launcher/releases/latest/download/latest.json`（仓库为公开仓库，使用 Release 资产中的 latest.json）。
- 更新公钥：`dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDZCRDUwMjM1QzIzRTVBRkIKUldUN1dqN0NOUUxWYTQ0WlRpUGZWeVg1V21OVDZna1BsdW9kSExSWitYaUNHay9TL2JZU0hkSnIK`
- 签名密钥已在本机生成（`~/.tauri/xjtu-launcher.key`，无密码）；私钥不入仓库，需自行备份并填入 GitHub Secret。丢失私钥后无法再为已发布客户端签名更新。
- 仓库 Variables 与 Secret 已于 2026-10-22 配置完毕，签名构建可跑通。

系统代码签名（Gatekeeper / SmartScreen）尚未配置：macOS 构建自 2026-10-29 起由工作流做 ad-hoc 签名（`APPLE_SIGNING_IDENTITY` 未配置时取 `-`），避免 0.5.1 出现的“已损坏，无法打开”；Developer ID 与公证仍未配置，首次安装仍会提示“无法验证开发者”，需要右键打开。Windows 仍无 Authenticode。步骤见 [SIGNING.md](SIGNING.md)。系统签名不影响自动更新的 minisign 校验。

## 每次发布的顺序

1. **开分支、定版本**：从 `main` 拉 `dev-v<版本>`（如 `dev-v0.6.0`），确认上一个版本可用。把 `package.json`、`package-lock.json`（两处）、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`、`src-tauri/tauri.conf.json` 改成同一个版本号；`tools/prepare-release.mjs` 会校验一致性，不一致直接报错。
2. **写版本记录**：按 [模板](../docs/releases/TEMPLATE.md) 新建 `docs/releases/v<版本>.md`，登记到 [索引](../docs/releases/README.md)，并把状态写成「最新」。「验证」一节先写计划，发布后补实际结果。自动发版会把这份文件当作 Release 正文，文件缺失或含 `YYYY-MM-DD` 占位符会中止发布。
3. **同步入口**：根 README 的三条下载链接改为新版本资产名（`v<版本>` 与文件名都含版本号），`dev-docs/CONVENTIONS.md` 的当前版本同步更新。
4. **本地验证**：`cargo test`、`npm test`、Clippy（`-D warnings`）、格式检查；再用发布配置构建一次，真机启动 10 秒以上。
5. **合入 `dev` 测试**：把 `dev-v<版本>` 合入 `dev` 并推送，交给测试人员验证真实登录、关键流程与安装包。未测试通过不得合入 `main`。
6. **合入 `main`**：测试通过后由 `dev` 创建 PR 合入 `main`。`main-guard` 会拒绝非 `dev` 来源的 PR；`main` 的规则禁止直接推送。
7. **自动发版**：`main` 更新后 `release.yml` 自动执行：版本与记录检查 → 测试与静态检查 → 三平台串行构建 → macOS 启动冒烟 → 完整资产与 `latest.json` 校验 → 正式发布并标记 Latest。已有同版本正式 Release 时跳过，所以同版本的其他 `main` 提交不会重复发版。
8. **发布后核对**：下载 Release 资产，检查 macOS 应用包签名（`codesign --verify --deep --strict`）、`Info.plist` 版本、双架构启动、DMG 校验和，以及全部资产的 `sha256` 与 GitHub 摘要一致。
9. **更新镜像**：上传资产到 `v<版本>/`，复制原清单、只替换 `platforms[*].url` 与 `notes`，覆盖 `latest.json`；旧版本目录保留。镜像没有自动化凭据，仍需手动执行。
10. **收尾**：把版本记录的「验证」改为实际结果，`dev-docs/DEVELOPMENT.md` 追加一条发布记录。
11. **撤回时**：删除 Release 与标签、删除镜像目录，把版本记录状态改为「已撤回」并写明原因与替代版本，再更新 README 与索引。

## 自动发版（`main` 更新）

`.github/workflows/release.yml` 在 `main` 推送、`v*` 标签推送和手动触发时运行：

- `prepare`：读取 `src-tauri/tauri.conf.json` 的版本，要求 `docs/releases/v<版本>.md` 存在且没有模板占位符；已有同版本正式 Release 时输出 `publish=false`，后续任务全部跳过。
- `verify`：在 Linux 上跑 `npm run format:check`、`npm test`、`npm run build`、`cargo test`、`cargo clippy -D warnings`。
- `release`：三平台串行构建，`tauri-action` 只写草稿，避免发布未齐的资产；Release 正文取版本记录文件。
- `publish`：等三平台全部成功后，校验 8 个安装包 / 更新包与 `.sig` 和 `latest.json`，再通过 GitHub API 把草稿改为正式 Release，并标记 Latest。

失败时草稿会保留，修复后重新运行工作流即可续传；不会出现只有部分平台就对外发布的正式版本。`workflow_dispatch` 可用于重跑。标签由 `tauri-action` 按版本创建，不再手工打标签。

## 首次部署

1. 使用 Tauri signer 生成发布密钥；私钥只存入发布系统的 Secret，公钥供客户端编译使用。
2. 更新地址目前写死在 `.github/workflows/release.yml`（P0 临时 HTTP），只需在仓库 Variables 中设置 `XJTU_UPDATE_PUBLIC_KEY`。可选设置 `XJTU_PLUGIN_SERVER`。
3. 在 Secrets 中设置 `TAURI_SIGNING_PRIVATE_KEY` 和相应的 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。不把私钥写入仓库。可选设置 `APPLE_SIGNING_IDENTITY`：留空时 macOS 用 `-` 做 ad-hoc 签名；拿到 Developer ID 证书后填证书名（配合 [SIGNING.md](SIGNING.md) 的导入步骤）。
4. 保持 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 的版本一致。
5. 运行 `Signed desktop release` 工作流（或直接合并到 `main`），构建 macOS Apple Silicon、macOS Intel 和 Windows x64，生成带签名的更新包、更新 JSON 与 Release。公钥配置必须已包含在用户当前安装的客户端中。

工作流把三个平台串行执行（`max-parallel: 1`）。`tauri-action` 无法按 tag 查询草稿，只能取发布列表里第一个同名草稿，并行 job 会各自新建草稿并把资产分散上传，导致每个草稿都缺平台、`latest.json` 只有部分 `platforms`。最终的 `publish` 任务会确认资产齐全后才发布，所以失败重跑时先保留草稿、补齐资产，不要手动发布残缺草稿。每次构建都会先读出已有 `latest.json` 再合并平台，挂到同一个草稿上即可得到完整清单。

工作流调用 `tools/prepare-release.mjs`，生成开启 `bundle.createUpdaterArtifacts` 的临时 Tauri 配置。普通 `npm run app:build` 保持可本地构建，无需发布密钥。更新相关环境变量也通过 Rust 编译进入客户端，不允许用户更改插件源时连带修改。

GitHub Releases 公共仓库可将更新端点配置为发布资产中的 latest.json；若仓库私有，应使用用户无需 GitHub 凭据即可访问的自有公开更新托管。更新清单中的平台名称、下载 URL 和 `.sig` 内容由官方 Tauri Action 生成。清单里的下载 URL 是 `api.github.com/.../releases/assets/{id}` 形式，客户端下载时会带 `Accept: application/octet-stream`，公开仓库正式发布后可匿名下载。签名构建已于 2026-10-22 跑通，但尚无正式发布的版本，跨版本更新仍未实测。

## 无法访问 GitHub 的用户 / 镜像更新源

应用更新默认读取内置更新地址。2026-10-25 起该地址临时写死为 `http://8.152.100.169/xjtu-launcher/latest.json`（P0，见 [P0.md](P0.md)）；用户在「应用设置 → 软件更新 → 更新源」仍可填写其他镜像地址（HTTP 或 HTTPS，不能含账号密码），留空则使用内置地址。

镜像需要同时提供：

1. `latest.json`，其中的 `platforms[*].url` 必须指向镜像自己的安装包，而不是 GitHub 的 `api.github.com/.../assets/...`；
2. 各平台安装包 / 更新包与 `.sig`，内容与签名必须和原发布一致。

只有镜像 `latest.json` 而下载地址仍指向 GitHub 时，检查更新能通过但下载仍会被网络挡住。镜像部署完成后，在客户端填入 `http://<镜像>/latest.json` 并点击「检查更新」即可验证。

### P0：当前使用 HTTP 镜像

在拿到域名和证书之前，内置更新地址使用 HTTP 直连 IP。安装包的 minisign 校验仍然有效，但清单本身可能被篡改，存在降级或拒绝服务风险。迁移步骤、代码位置和退出条件见 [P0.md](P0.md)。恢复 HTTPS 后，必须同时收紧 `src-tauri/src/config.rs`、`src/components/AppUpdateSettings.vue` 和 `tools/prepare-release.mjs` 三处校验。

HTTP 端点必须在发布配置里带上 `dangerousInsecureTransportProtocol: true`，否则 release 构建在插件初始化阶段直接退出（0.5.0 即因此无法启动，见 [P0.md](P0.md)）。`tools/prepare-release.mjs` 已按端点协议自动写入。发布前必须用 release 配置实跑一次：macOS 直接启动 `XJTU Launcher.app/Contents/MacOS/xjtu-launcher`，确认进程存活再打标签；工作流在 macOS 构建后也会执行同样的 10 秒启动检查。Windows 仍无法在 CI 里可靠启动，发布前必须在真机安装并打开一次。

### 当前镜像内容（2026-10-30）

服务器 `8.152.100.169` 上由 nginx 直接提供静态文件：

- `/srv/xjtu-launcher/public/latest.json`：改写后的清单，`platforms[*].url` 指向本镜像的 `v0.5.2/` 目录，`notes` 与 Release 说明一致；`signature` 原样保留。
- `/srv/xjtu-launcher/public/v0.5.2/`：两个 `.dmg`、两个 `.app.tar.gz`、`_x64-setup.exe` 及各自的 `.sig`。

上线时逐个文件与 GitHub Release 的 `sha256` 摘要比对通过，从镜像完整下载后哈希同样一致；`latest.json` 返回 `200`、`application/json`，安装包支持 `Range`（`206`）。服务器出口带宽约 3.4 Mbps，25 MB 的更新包大约需要一分钟。0.5.1 因 macOS 包签名缺陷已撤回，Release、标签与镜像目录都已删除；0.5.0 同样已撤回。

下一个版本发布后，把新资产上传到 `v<版本>/`，复制原清单、只替换 `platforms[*].url` 与 `notes`，再覆盖 `latest.json`。旧版本目录保留，已发布客户端仍可能请求。

## 发布后

- 更新 README「下载安装」里的三条直达链接到新版本的资产名（`vX.Y.Z` 与文件名都含版本号）；否则首页按钮会指向旧版本。
- 在 GitHub Releases 页面确认新版本已标记为 Latest。
- 把 `docs/releases/v<版本>.md` 的「验证」改为实际结果，登记到索引并在 `docs/README.md` 指向最新版本。

## 运行行为

启动后按用户选项自动检查；手动检查位于“应用设置 → 软件更新”。下载与状态保存在 Rust 运行时，切换页面不会中断。安装与重启前尝试取得校园任务、账号操作和插件安装锁，已有操作未结束则提示稍后重试。macOS 安装完成后由用户选择重启，Windows 由安装程序按 Updater 机制退出并更新。

未来 Agent 有实际运行任务后，必须把任务锁接入更新前检查，不能宣称目前已经覆盖尚不存在的 Agent 运行时。

参考：Tauri Updater 官方文档与 `tauri-apps/tauri-action` 官方 README。未使用真实发布包进行跨版本安装，Windows 安装行为仍需 Windows 实机验收。
