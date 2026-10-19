# XJTU Launcher

面向学生的本地校园工作台，使用 **Tauri 2 + Rust + Vue 3 + TypeScript**，目标平台为 macOS 和 Windows。

## 当前开发版

本轮用户明确授权直接实现五项客户端反馈。当前包含：

- 主页、可收起侧栏、本地插件管理、完整课表和校园导航；系统原生标题栏。
- 左下角头像进入个人页：本机显示名称、个人头像、主页壁纸，以及校园账号管理入口。
- 独立应用设置：减少动态效果、时钟秒数显示、多厂商模型配置。
- 模型配置支持名称、协议、接口地址、模型、独立 API Key，以及新增、编辑、删除和默认项。密钥保存在 AES-GCM 加密文件，主密钥由系统凭据库管理；可主动导入旧版单个 Key。这里只配置服务，Agent 模型调用仍属 V0.2。
- 社区入口与未接入状态；尚未连接 Web 论坛。
- 侧栏顶部留白，主页下方恢复 Agent 入口。图标点击反馈、文字渐隐与页面切换过渡；点击立即更新，串行保存配置。
- 侧栏可拖动分隔线调整展开宽度并保存，双击恢复默认；保留 56px 收起宽度和 Cmd/Ctrl + 反斜杠。
- 个人显示名称默认展示文字，点击后编辑。应用设置按“外观与交互 / 主页 / 模型服务”分类；Agent 入口暂显示真实未接入状态。

Rust 已有本地多账号、校园 CAS / WebVPN 状态机和课表 provider。校园账号页支持管理已有账号；添加账号表单仍未实现，被否决的弹窗没有恢复。真实学校账号尚未验收。

## 启动桌面应用

需要 Node.js 22.12+、Rust 和对应平台的 Tauri 构建环境。

```sh
npm ci
npm run dev
```

`npm run dev` 启动 **Tauri 桌面窗口**。开发期间 Vite 仅作为窗口内部的资源服务器；不将浏览器页面作为产品或验收入口。浏览器模拟配置和模拟插件数据已删除。

打包独立应用：

```sh
npm run app:build
```

macOS 本地快速打包开发版：

```sh
npm run tauri -- build --debug --bundles app
```

产物为 `src-tauri/target/debug/bundle/macos/XJTU Launcher.app`，可直接双击运行，不需要 Vite、浏览器或本地 HTTP 服务。正式包在 release 目录。Windows 安装包需在 Windows 构建和验证。

若终端找不到 cargo，在 macOS 执行 `source "$HOME/.cargo/env"`。

## 检查

```sh
npm test
npm run build
npm run format:check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

当前已完成 macOS 开发包构建和原生窗口检查；未完成 Windows 实机、真实校园登录及正式签名公证。

## 文档与版本

- [产品要求与讨论上下文](docs/PROJECT_CONTEXT.md)
- [V0.1 实现与验收](docs/V0.1.md)
- [本地插件协议](docs/PLUGINS.md)
- [校园参考项目](docs/REFERENCE_PROJECT.md)
- [桌面界面参考依据](docs/design/DESKTOP_REFERENCES.md)

V0.1 聚焦启动器、校园账户、本地插件和课表；V0.2 实现 Agent、MCP、Skill、长期记忆；V0.3 实现在线插件市场和完整主页 DIY。未建设启动器平台账户服务端，插件市场服务端可以后续建设。

页面参考与授权记录见 [设计评审](docs/design/PAGES_REVIEW.md)。本轮授权仅适用于已明确的五项客户端反馈。
