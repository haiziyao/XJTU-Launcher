# XJTU Launcher

**把交大人的校园生活，收进一个桌面应用。**

校园账号、课表、成绩、考勤、校园卡、空闲教室、评教，以及可以调用这些能力的 Agent——都放在同一个窗口里。

[![最新版本](https://img.shields.io/github/v/release/haiziyao/XJTU-Launcher?label=%E4%B8%8B%E8%BD%BD&style=flat-square)](https://github.com/haiziyao/XJTU-Launcher/releases/latest)
[![MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

## 它是什么

XJTU Launcher 是面向西安交通大学学生的桌面校园工作台。不用再在浏览器标签、教务网站和各类小程序之间来回切换：登录一次校园账号，课表、考试、成绩、考勤、校园卡、体测、空闲教室、全校课程、校历和评教都在这一个应用里完成。

它不是把网页塞进客户端，而是用 Rust 直接对接校园系统的协议：会话留在本机，查询走原生请求，界面用桌面应用的方式重新组织。少一点“打开网页—登录—找入口”，多一点“想查什么，直接就在这里”。

## 愿景

我们想做的不是一个“功能更多的工具箱”，而是一个**真正属于交大人的桌面工作台**：

- **一处登录，处处可用。** 校园账号只有一个入口，直连和 WebVPN 自动切换；教务、考勤、校园卡这些各自为政的系统，在应用里表现为同一套体验。
- **本地优先。** 账号、密码、会话和数据尽量留在你自己的电脑上，不依赖启动器的账号服务器，也不把校园凭据传到云端。
- **可扩展，而不是越做越臃肿。** 校园功能做成插件，桌面主题、主页元素也可以定制；你需要什么就装什么，不需要的可以关掉。
- **让 Agent 真正懂校园。** 不是一个通用聊天框，而是能查你的课表、成绩和考勤，并清楚知道自己能力边界、把写操作交还给用户的助手。

目标很简单：让交大人的日常校园操作，从“完成任务”变成“顺手的事”。

## 主要能力

**校园**

- 校园账号：多账号、验证码与短信验证、WebVPN 自动回退，可选保存密码
- 课表：今日课程与完整学期课表，近校园网自动刷新
- 畅行校园：成绩、考试安排、考勤流水、校园卡余额与流水、体测成绩、空闲教室、全校课程、校历、评教（预览后由你确认提交）
- 校园导航：在独立窗口里复用登录状态访问校内网站

**Agent**

- 流式对话，可调用校园查询与已启用的 MCP 工具
- 对话历史、置顶与工作区管理
- 角色、Skill、预设任务；输入框旁有上下文用量圆环与厂商余额
- 多厂商模型接入：模型在对话框里选择，设置页负责获取和查看模型列表

**给编程 Agent 的接口**

- 个人页可打开本机 MCP 模式，Claude Code 等编程 Agent 通过 `http://127.0.0.1:39217/mcp` 调用课表、成绩、考试、校园卡等查询（端口可自行修改）
- 逐项勾选开放哪些工具；“查看工具”按类别列出名称、说明与可达性，并能实际检查校园会话
- 仅监听本机回环地址，不经过启动器账号；当前开放的是查询工具，后续可以加入写操作

**桌面体验**

- 原生标题栏、可收起和拖宽的侧栏、主题色、壁纸与可点击的侧栏宠物
- “主页主题”插件：时钟风格与主页元素开关
- 插件市场：安装、启用、排序与更新；自动更新走 Tauri Updater

## 下载安装

先确认你的电脑，再点对应的链接，下载后直接安装即可：

- 🍎 **Mac，Apple 芯片**（M1 / M2 / M3 / M4，2020 年之后的机型）→ **[点击下载 .dmg](https://github.com/haiziyao/XJTU-Launcher/releases/download/v0.3.1/XJTU.Launcher_0.3.1_aarch64.dmg)**
- 🍎 **Mac，Intel 芯片**（2020 年之前的机型）→ **[点击下载 .dmg](https://github.com/haiziyao/XJTU-Launcher/releases/download/v0.3.1/XJTU.Launcher_0.3.1_x64.dmg)**
- 🪟 **Windows 64 位**（Windows 10 / 11）→ **[点击下载 .exe](https://github.com/haiziyao/XJTU-Launcher/releases/download/v0.3.1/XJTU.Launcher_0.3.1_x64-setup.exe)**

不确定自己是什么芯片？Mac 左上角 →「关于本机」，看“芯片”一栏：写着 Apple M 系列就选第一个，写着 Intel 就选第二个。

其他版本、更新说明和历史安装包都在 **[Releases 页面](https://github.com/haiziyao/XJTU-Launcher/releases)**。

> 安装包目前**没有系统代码签名**：macOS 首次打开可能提示“无法验证开发者”（右键 →「打开」即可），Windows 可能出现 SmartScreen 提示（「更多信息」→「仍要运行」）。签名方案与进度见 [docs/SIGNING.md](docs/SIGNING.md)。macOS 自动更新已实测通过（0.1.1 → 0.3.1），Windows 安装与升级仍待实机验收。

## 开发

技术栈为 **Rust + Tauri 2 + Vue 3 + TypeScript**。环境准备、构建、测试与提交约定都在开发文档里：

- [开发指南](docs/DEVELOPMENT.md)
- [文档索引](docs/README.md)
- [约定手册](docs/CONVENTIONS.md)

## 致谢

特别感谢 [**XJTUToolBox（仙交百宝箱）**](https://github.com/yan-xiaoo/XJTUToolBox)：一个基于 PyQt5 的优秀校园工具箱，覆盖课表、考勤、评教、成绩、思源学堂等场景。本项目的校园协议、接口行为与交互思路大量参考自它，没有它就没有今天的 XJTU Launcher。强烈推荐前往仓库 Star、关注并参与贡献：[yan-xiaoo/XJTUToolBox](https://github.com/yan-xiaoo/XJTUToolBox)。

XJTUToolBox 以 **GPL-3.0** 许可证发布；本项目按其公开行为独立用 Rust 重新实现，不复制其源码，也不将其作为运行时依赖。若未来直接复用其代码，将按其许可证要求处理。

同时感谢 [Tauri](https://tauri.app/)、[Vue](https://vuejs.org/)、[lucide](https://lucide.dev/)、[Noto Sans SC](https://fonts.google.com/noto/specimen/Noto+Sans+SC) 等开源项目。

## 许可证

本项目以 [MIT](LICENSE) 许可证开源，Copyright (c) 2026 haiziyao。

内嵌的 Noto Sans SC 字体按其 SIL Open Font License 1.1 分发，许可证文本见 `src/assets/fonts/OFL.txt`。
