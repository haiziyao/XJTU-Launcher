# XJTU Launcher

面向西安交通大学学生的桌面校园工作台，把校园账号、课表、常用网站和插件集中在一个应用中。

使用 **Rust + Tauri 2 + Vue 3 + TypeScript** 开发，目标平台为 **macOS 和 Windows**。当前版本为 **0.1.0 开发版**，已在 macOS 构建运行，Windows 尚待实机验收。

## 当前功能

| 模块 | 已实现 |
| --- | --- |
| 桌面工作台 | 原生窗口、可收起和拖动调整宽度的侧栏、个人头像与主页壁纸 |
| 校园账号 | 本地多账号、账号密码登录、验证码与短信验证、WebVPN 回退、可选保存密码 |
| 课表 | 读取当前账号课表，查看今日课程与完整课表 |
| 校园导航 | 在独立校园窗口中复用账号会话，按需通过 WebVPN 访问 |
| 插件市场 | 校园功能、远端市场、已安装三个分区；导入、下载、更新、移除及排序 |
| 插件管理 | 启用与侧栏显示分别控制；修改插件服务地址前需确认 |
| 模型设置 | 多厂商、API 地址、模型和 API Key，包含 DeepSeek 预设 |
| 软件更新 | Tauri Updater 检查、下载和安装流程；正式更新服务仍待配置 |

校园登录和真实课表已通过用户实测。部分校园网站仍可能要求重新认证。

**尚未完成：** Agent 运行时、MCP、Skill、长期记忆、Web 社区同步，以及更多校园功能页面。本科成绩、考试、空教室和评教等已有部分 Rust 适配，但尚未完成页面接入与真实业务验收。模型设置可保存配置，当前还不能在 Agent 中对话。

## 启动应用

准备 Node.js 22.12+、Rust 工具链和对应平台的 Tauri 构建依赖：macOS 使用 Xcode Command Line Tools，Windows 使用 Visual Studio C++ Build Tools 与 WebView2。

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

Windows 安装包在 Windows 上构建，产物位于 `src-tauri/target/release/bundle/`。若终端找不到 `cargo`，先加载 Rust 环境，例如 macOS 下执行 `source "$HOME/.cargo/env"`。

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

应用更新使用独立的发布地址与签名公钥。开发构建尚未配置正式更新源，设置页会如实显示未配置。

## 分支与开发

- **`main`**：应用源码、使用说明和插件协议。
- **`dev`**：基于 main 继续开发，包含开发指南、设计资料和约定手册。

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
