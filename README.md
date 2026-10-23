# XJTU Launcher

> 下载安装：[最新版本](https://github.com/haiziyao/XJTU-Launcher/releases/latest) · 开发入口：[文档索引](docs/README.md) · [开发指南](docs/DEVELOPMENT.md) · [约定手册](docs/CONVENTIONS.md)。

面向西安交通大学学生的桌面校园工作台，把校园账号、课表、Agent、常用网站和插件集中在一个应用中。

使用 **Rust + Tauri 2 + Vue 3 + TypeScript** 开发，目标平台为 **macOS 和 Windows**。当前版本为 **0.3.1**，可从 [Releases](https://github.com/haiziyao/XJTU-Launcher/releases/latest) 下载 macOS（Apple Silicon / Intel）与 Windows 安装包；macOS 自动更新已实测通过，Windows 尚待实机验收。

校园协议与接口行为大量参考自 [XJTUToolBox](https://github.com/yan-xiaoo/XJTUToolBox)，详见文末[致谢](#致谢)。

## 当前功能

| 模块       | 已实现                                                                                                                                         |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| 桌面工作台 | 原生窗口、可收起和拖动调整宽度的侧栏、主题色、可点击的侧栏宠物（自检校园会话）、个人头像与全窗口壁纸；内置“主页主题”插件调整时钟风格和主页元素 |
| 校园账号   | 本地多账号、账号密码登录、验证码与短信验证、WebVPN 回退、可选保存密码                                                                          |
| 课表       | 读取当前账号课表，查看今日课程与完整课表                                                                                                       |
| 畅行校园   | 成绩、考试、考勤、校园卡、体测、空闲教室、全校课程、校历、评教（预览后由用户确认提交）；常用工具可置顶                                         |
| 校园导航   | 在独立校园窗口中复用账号会话，按需通过 WebVPN 访问                                                                                             |
| 插件市场   | 校园功能、远端市场、已安装三个分区；导入、下载、更新、移除及排序                                                                               |
| 插件管理   | 只保留启用开关，侧栏入口跟随启用状态；修改插件服务地址前需确认                                                                                 |
| Agent      | 流式对话、工具调用（校园 + MCP）、对话历史与工作区、角色 / Skill / 预设任务、上下文用量与余额（DeepSeek）                                      |
| 模型设置   | 多厂商、API 地址与 API Key（可查看/替换），按厂商获取模型列表；具体模型在 Agent 对话框中选择，支持连通性测试                                   |
| 软件更新   | Tauri Updater 检查、下载和安装；macOS 0.1.1 → 0.3.1 实测通过，系统代码签名尚未配置                                                             |

校园登录和真实课表已通过用户实测。部分校园网站仍可能要求重新认证。

**尚未完成：** Windows 实机验收、系统代码签名与公证、图书馆 / 场馆 / 学籍 / LMS 页面、Web 社区同步，以及更多 MCP / Skill 场景。部分校园业务（如空闲教室、全校课程）仍需真实账号逐项验收。

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
3. 在插件市场中管理功能的启用状态，启用后侧栏出现对应入口。
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

## 致谢

特别感谢 [**XJTUToolBox（仙交百宝箱）**](https://github.com/yan-xiaoo/XJTUToolBox)：这是一个基于 PyQt5 的优秀校园工具箱，覆盖课表、考勤、评教、成绩、思源学堂等场景。本项目的校园协议、接口行为与交互思路大量参考自该项目，没有它就没有今天的 XJTU Launcher。强烈推荐前往仓库 Star、关注并参与贡献：[yan-xiaoo/XJTUToolBox](https://github.com/yan-xiaoo/XJTUToolBox)。

XJTUToolBox 以 **GPL-3.0** 许可证发布；本项目按其公开行为独立用 Rust 重新实现，不复制其源码，也不将其作为运行时依赖。若未来直接复用其代码，将按其许可证要求处理。

同时感谢 [Tauri](https://tauri.app/)、[Vue](https://vuejs.org/)、[lucide](https://lucide.dev/)、[Noto Sans SC](https://fonts.google.com/noto/specimen/Noto+Sans+SC) 等开源项目。

## 许可证

本项目以 [MIT](LICENSE) 许可证开源，Copyright (c) 2026 haiziyao。

内嵌的 Noto Sans SC 字体按其 SIL Open Font License 1.1 分发，许可证文本见 `src/assets/fonts/OFL.txt`。
