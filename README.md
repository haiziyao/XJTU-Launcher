# XJTU Launcher

面向西安交通大学学生的桌面校园工作台，把校园账号、课表、常用网站和插件集中在一个应用中。

使用 **Rust + Tauri 2 + Vue 3 + TypeScript** 开发，支持 **macOS、Windows 和 Linux**。当前公开版本为 **v0.1.0**，三平台构建和自动检查均已通过；Windows / Linux 的真实校园登录与课表仍待用户环境验收。

## 下载安装

**[最新版本与全部安装包](https://github.com/haiziyao/XJTU-Launcher/releases/latest)** · [v0.1.0 发布说明](docs/releases/v0.1.0.md)

| 系统                | v0.1.0 下载                                                                                                                                                                                                                              | 要求                                                          |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| Windows x64         | [exe 安装器（推荐）](https://github.com/haiziyao/XJTU-Launcher/releases/download/v0.1.0/XJTU.Launcher_0.1.0_x64-setup.exe) / [msi](https://github.com/haiziyao/XJTU-Launcher/releases/download/v0.1.0/XJTU.Launcher_0.1.0_x64_zh-CN.msi) | Windows 10 / 11；缺少 WebView2 时安装器会联网安装             |
| macOS Apple Silicon | [dmg](https://github.com/haiziyao/XJTU-Launcher/releases/download/v0.1.0/XJTU.Launcher_0.1.0_aarch64.dmg)                                                                                                                                | macOS 12+，M 系列芯片                                         |
| macOS Intel         | [dmg](https://github.com/haiziyao/XJTU-Launcher/releases/download/v0.1.0/XJTU.Launcher_0.1.0_x64.dmg)                                                                                                                                    | macOS 12+，Intel 芯片                                         |
| Linux x64           | [deb](https://github.com/haiziyao/XJTU-Launcher/releases/download/v0.1.0/XJTU.Launcher_0.1.0_amd64.deb) / [AppImage](https://github.com/haiziyao/XJTU-Launcher/releases/download/v0.1.0/XJTU.Launcher_0.1.0_amd64.AppImage)              | Ubuntu 22.04+；AppImage 需添加可执行权限，部分系统需要 FUSE 2 |

安装包附带 [SHA-256 校验文件](https://github.com/haiziyao/XJTU-Launcher/releases/download/v0.1.0/SHA256SUMS.txt)。首版尚未配置 Windows / Apple 开发者代码签名及 macOS 公证，系统可能提示未验证的发布者；macOS 可在“系统设置 → 隐私与安全性”中确认允许打开。当前通过 Release 手动下载更新。

## 当前功能

| 模块       | 已实现                                                                     |
| ---------- | -------------------------------------------------------------------------- |
| 桌面工作台 | 原生窗口、可收起和拖动调整宽度的侧栏、个人头像与主页壁纸                   |
| 校园账号   | 本地多账号、账号密码登录、验证码与短信验证、WebVPN 回退、可选保存密码      |
| 课表       | 读取当前账号课表，查看今日课程与完整课表                                   |
| 校园导航   | 在独立校园窗口中复用账号会话，按需通过 WebVPN 访问                         |
| 插件市场   | 校园功能、远端市场、已安装三个分区；导入、下载、更新、移除及排序           |
| 插件管理   | 启用与侧栏显示分别控制；修改插件服务地址前需确认                           |
| 模型设置   | 多厂商、API 地址、模型和 API Key，包含 DeepSeek 预设                       |
| 软件更新   | 已提供 GitHub Release 下载；应用内更新流程已实现，正式更新源与签名仍待配置 |

校园登录和真实课表已在 macOS 通过用户实测。部分校园网站仍可能要求重新认证。

**尚未完成：** Agent 运行时、MCP、Skill、长期记忆、Web 社区同步，以及更多校园功能页面。本科成绩、考试、空教室和评教等已有部分 Rust 适配，但尚未完成页面接入与真实业务验收。模型设置可保存配置，当前还不能在 Agent 中对话。

## 从源码开发

准备 Node.js 24（与 `.nvmrc` 和 CI 一致）、Rust stable 工具链，以及对应平台的构建依赖：

- macOS：Xcode Command Line Tools。
- Windows：Visual Studio C++ Build Tools、Windows SDK 与 WebView2。
- Linux（Ubuntu 22.04）：

```sh
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libfuse2 xdg-utils
```

启动桌面应用：

```sh
npm ci
npm run dev
```

命令启动 Tauri 桌面应用。开发时 Vite 提供界面资源；打包后的应用可以独立运行。

构建安装包：

```sh
npm run app:build
```

仅构建 macOS App：

```sh
npm run app:build -- --bundles app
open "src-tauri/target/release/bundle/macos/XJTU Launcher.app"
```

各平台安装包在对应系统上构建，产物位于 `src-tauri/target/release/bundle/`。若终端找不到 `cargo`，先加载 Rust 环境，例如 macOS 下执行 `source "$HOME/.cargo/env"`。版本标签会触发 GitHub Actions 自动构建并发布三平台安装包，详见 [版本发布](docs/RELEASING.md)。

## 使用

1. 点击左下角个人头像，进入“校园账号”，添加并登录学校账号。应用会检测直连可用性，必要时进入 WebVPN 流程。
2. 打开课表或校园导航；账号信息与选择保存的密码保留在本机。
3. 在插件市场中管理功能。“启用”控制插件是否可用，“侧栏显示”仅控制导航入口。
4. 在应用设置中调整外观、模型服务、插件来源和软件更新。

校园账号、密码、会话与模型 API Key 按本地明文 JSON 保存于 `~/.xjtu-launcher/auth.json`；日常读写不使用系统钥匙串。旧加密数据仅在迁移时读取旧凭据。普通界面配置和插件使用系统应用数据目录。

项目不要求注册启动器平台账号，也不提供校园凭据的云端保存服务。

## 插件

当前外部插件支持**声明式导航页面**，内置校园能力由 Rust 提供。动态脚本、自定义可执行插件和 Agent 工具运行时仍在后续开发范围。

- [插件格式与使用](docs/PLUGINS.md)
- [本地插件示例](examples/campus-links/plugin.json)
- [插件目录服务说明](tools/plugin-server/README.md)

本地运行目录服务：

```sh
npm run market:serve
```

在“应用设置 → 插件服务”中确认更改地址为 `http://127.0.0.1:17321`，即可下载安装示例插件。正式插件服务器尚未部署；客户端不会默认连接测试服务器或设计稿中的示例域名。

应用内更新需要单独配置发布地址与签名公钥。v0.1.0 尚未配置正式更新源，设置页会如实显示未配置；请从 GitHub Releases 下载更新。

## 分支与开发

- **`main`**：包含已发布的跨平台应用源码、构建发布流程、使用说明与插件协议。
- **`dev`**：保留开发指南、设计资料和约定手册，供后续开发使用。
- **`codex/windows-support`**：Windows 适配分支，保留平台适配与验证入口；v0.1.0 的发布改动已同步到 main。

已合入主线的旧功能分支会及时清理；已发布源码由版本标签固定保存。

开发资料入口：[dev 分支文档](https://github.com/haiziyao/XJTU-Launcher/tree/dev/docs)、[开发指南](https://github.com/haiziyao/XJTU-Launcher/blob/dev/docs/DEVELOPMENT.md)、[约定手册](https://github.com/haiziyao/XJTU-Launcher/blob/dev/docs/CONVENTIONS.md)。

本地检查：

```sh
npm test
npm run build
npm run format:check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## 参考

校园业务优先参考 [XJTUToolBox](https://github.com/yan-xiaoo/XJTUToolBox) 的协议与交互，再按本项目的 Rust / Tauri 架构适配。参考项目不作为运行时依赖；涉及实际代码复用时需遵守原项目许可证。
