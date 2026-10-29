# XJTU Launcher

**把交大人的校园生活，收进一个桌面应用。**

校园账号、课表、成绩、考勤、校园卡、空闲教室、图书馆、场馆、思源学堂、评教，以及可以调用这些能力的 Agent 和本机 MCP 接口——都放在同一个窗口里。

[![最新版本](https://img.shields.io/github/v/release/haiziyao/XJTU-Launcher?label=%E4%B8%8B%E8%BD%BD&style=flat-square)](https://github.com/haiziyao/XJTU-Launcher/releases/latest)
[![MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

<p align="center"><img src="docs/screenshots/home.png" alt="主页：问候、今日课程、校园快捷入口与插件开关" width="900"></p>

## 它是什么

XJTU Launcher 是面向西安交通大学学生的桌面校园工作台。不用再在浏览器标签、教务网站和各类小程序之间来回切换：登录一次校园账号，课表、考试、成绩、考勤、校园卡、体测、空闲教室、全校课程、校历、评教、图书馆、场馆和思源学堂都在这一个应用里完成。

它不是把网页塞进客户端，而是用 Rust 直接对接校园系统的协议：会话留在本机，查询走原生请求，界面用桌面应用的方式重新组织。少一点“打开网页—登录—找入口”，多一点“想查什么，直接就在这里”。

应用本身也是一个可以扩展的桌面工作台：校园功能做成插件，主题、主页元素可以定制；内置的 Agent 能查询校园数据；打开本机 MCP 模式后，Claude Code 等编程 Agent 也能直接调用同一批校园工具。

## 主要能力

### 校园账号与课表

- **一处登录**：账号密码、手机号短信验证码、扫码登录三种方式；多账号保存、图片验证码与短信验证、可把本机登记为可信设备（之后一般不再要求短信）、WebVPN 自动回退，密码可选保存；凭据只存在本机的 `~/.xjtu-launcher/auth.json`
- **课表**：今日课程与完整学期课表，按周切换；近校园网自动刷新，登录状态失效时提示重新连接
- **课表自定义**：点课程块修改名称、教师、地点、周次、节次与颜色，可隐藏课程，也能添加只存在本机的自建课程；改动只保存在本机，随时可恢复学校原始内容
- **校园导航**：需要查看原网站时，在独立窗口里复用登录状态打开一网通办、教务系统、思源学堂和图书馆，不用重新登录，也不会影响主窗口
- **主页通知**：主页默认展示可上下滚动的教务处教学通知，点击在校园窗口打开原文；可在「主页主题」里换成插件区或其他元素

### 学习

- **成绩查询**：按学期查看课程成绩与绩点
- **考试安排**：考试时间、地点与考场
- **空闲教室**：按日期与节次查询可用自习教室
- **全校课程**：按学期和课程名检索开课信息
- **校历**：学年、学期与假期安排
- **学籍档案**：个人学籍信息
- **一键评教**：先预览评教内容，确认后由你主动提交；结果不明时不会自动重试

### 校园生活

- **考勤查询**：刷卡流水与课程考勤记录
- **校园卡**：余额与最近 30 天流水
- **体测成绩**：体测项目与总分
- **图书馆**：楼层、区域与实时座位
- **场馆预约**：场馆时段与我的预订
- **思源学堂**：课程按学期分组，作业 / 学习资料 / 课程回放 / 在线视频 / 讨论等按类型切换；活动详情可查看附件、另存文件，录播课可直接获取回放地址；Agent 可查看作业提交记录、分数与教师批语，也能上传作业附件并提交（先预览、再确认）

<p align="center"><img src="docs/screenshots/campus.png" alt="畅行校园：按学习与校园生活分类的校园工具" width="900"></p>

### Agent

- **流式对话**：回答过程中可停止；对话在本机持久保存，切换页面不会中断。回复按 Markdown 渲染，代码、列表、表格和链接都能正常显示；右侧刻度对应每一轮提问，悬停预览、点击跳转
- **工具循环**：可以查询课表、成绩、考试、考勤、校园卡、体测、学籍、空闲教室、全校课程、校历、评教列表、思源学堂、图书馆与场馆，也能读取教务处、学生处、研究生院与网络信息中心的公开通知；改课表这类本机操作先给预览，你确认后才生效；还会调用你在 Agent 设置里启用的 MCP 工具
- **思源学堂作业**：说清要交的作业和本地文件后，Agent 先返回课程、作业、截止时间、文件与文字的预览；你明确同意后才上传附件并提交。提交结果不明时不会自动重试
- **评教**：Agent 可以自动填写待评问卷，并给某位老师单独指定评语；先返回逐门课程的预览，你说提交后才真正提交。结果不明时不会自动重试
- **对话管理**：历史记录、置顶、搜索、重命名、删除与工作区分组
- **角色与 Skill**：内置角色、预设任务和 Skill 目录；输入框旁用 `+` 选择角色、`@` 引用 Skill
- **全局记忆**：自动注入系统提示词，可手动维护，也能由模型写入
- **多厂商模型**：兼容 OpenAI 兼容接口、Anthropic 与 Gemini；模型在对话框里选择，输入框显示上下文用量圆环与厂商余额

### 本机 MCP 接口

个人页可以打开 **MCP 模式**，把校园能力直接开放给 Claude Code 等编程 Agent：

- 在 `http://127.0.0.1:39217/mcp` 提供 MCP 服务，端口可自行修改，应用失败会回到原端口并说明原因
- 内置 14 个校园工具：只读查询按领域合并，用 `action` 参数选择具体查询（学期、成绩、考试、校历、学籍、全校课程等），写操作保持独立；覆盖教务、教室与校历、考勤、校园卡、体测、评教、思源学堂、图书馆与场馆，以及教务处、学生处、研究生院与网络信息中心的公开通知，按分类展示名称、说明与当前可达性
- 逐项勾选开放哪些工具；未勾选的工具不会出现在 `tools/list` 中
- 一键复制地址与 `claude mcp add` 命令，也能实际检查校园会话是否可用
- 仅监听本机回环地址，不经过启动器账号。写工具只有三个：`campus_lms_submit` 默认只预览作业，`confirm=true` 才上传并提交；`campus_evaluations_submit` 必须先由 `campus_evaluation` 的 `action=preview` 生成预览，再带 `confirm=true` 提交，结果不明时不重试；`campus_timetable_edit` 只改本机课表，默认返回改动前后的对照，用户确认后再带 `confirm=true`。接口无请求认证，请勿把端口暴露到局域网

<p align="center"><img src="docs/screenshots/mcp.png" alt="个人页 MCP 模式：端口、地址、复制命令与工具数量" width="900"></p>

<p align="center"><img src="docs/screenshots/mcp-tools.png" alt="MCP 工具清单：按分类展示工具名称、说明与可达性" width="900"></p>

### 插件与市场

- 课表、畅行校园、Agent 设置、主页主题都以插件形式出现，侧栏入口跟随启用状态
- 应用更新走 Tauri Updater：启动检查、Markdown 更新说明、下载进度与安装重启；发现新版本会在侧栏出现入口，也可以手动检查。可填写镜像更新源，留空使用内置地址
- 插件市场分为校园功能、远端市场与已安装三个分区：安装、启用、排序、更新与移除
- 远端目录支持地址确认与危险提示；声明式插件无需随应用更新即可安装
- 插件也可以单独检查更新

### 桌面体验

- 系统原生标题栏、可收起与拖宽的侧栏、六只各带独立配色的侧栏宠物
- **最小化桌宠**：主窗口最小化后出现浮动桌宠和通知横条（关闭窗口仍然是关闭）；四只内置角色、玻璃 / 深色玻璃 / 纯白三种横条风格；横条显示校园通知、Agent 任务完成与更新提示；点击横条还原应用，双击桌宠可选择「只保留桌宠」或打开设置
- 主题色、自定义壁纸、主页时钟风格、主页分区开关与显示顺序
- 减少动态效果选项，尊重系统的“减弱动态效果”设置
- 课表、成绩、校园卡等页面先显示上次结果，再在后台取回最新数据；缓存按校园账号隔离

## 下载安装

先确认你的电脑，再点对应的链接，下载后直接安装即可：

- 🍎 **Mac，Apple 芯片**（M1 / M2 / M3 / M4，2020 年之后的机型）→ **[点击下载 .dmg](https://github.com/haiziyao/XJTU-Launcher/releases/download/v0.5.1/XJTU.Launcher_0.5.1_aarch64.dmg)**
- 🍎 **Mac，Intel 芯片**（2020 年之前的机型）→ **[点击下载 .dmg](https://github.com/haiziyao/XJTU-Launcher/releases/download/v0.5.1/XJTU.Launcher_0.5.1_x64.dmg)**
- 🪟 **Windows 64 位**（Windows 10 / 11）→ **[点击下载 .exe](https://github.com/haiziyao/XJTU-Launcher/releases/download/v0.5.1/XJTU.Launcher_0.5.1_x64-setup.exe)**

不确定自己是什么芯片？Mac 左上角 →「关于本机」，看“芯片”一栏：写着 Apple M 系列就选第一个，写着 Intel 就选第二个。

> ⚠️ **系统要求**：macOS 12+ 或 Windows 10 / 11；Windows 需要 WebView2（Windows 11 已内置，缺失时安装程序会下载安装）。

> 校园登录报「未收到预期的校园认证页面，请重新开始」，且括号里没有主机名？这是 **0.3.3 及更早版本**的提示，学校登录页改版后旧版会误判 WebVPN 登录结果。请重新下载上面的 0.5.1 安装包覆盖安装；0.4.0 起已修复，新版报错会带上实际落地的网站域名，便于反馈。

其他版本、更新说明和历史安装包都在 **[Releases 页面](https://github.com/haiziyao/XJTU-Launcher/releases)**。

> **macOS 提示“已损坏，无法打开”**：0.5.1 及更早的 macOS 安装包只有二进制签名，应用包本身没有签名，Gatekeeper 会判定为损坏。临时处理：
>
> ```bash
> sudo codesign --force --deep --sign - "/Applications/XJTU Launcher.app"
> sudo xattr -dr com.apple.quarantine "/Applications/XJTU Launcher.app"
> ```
>
> 后续构建已改为 ad-hoc 签名，不会再报“已损坏”。安装包仍**没有 Developer ID 与公证**：首次打开可能提示“无法验证开发者”，右键 →「打开」，或到“系统设置 → 隐私与安全性”点「仍要打开」。Windows 可能出现 SmartScreen 提示（「更多信息」→「仍要运行」）。签名方案与进度见 [dev-docs/SIGNING.md](dev-docs/SIGNING.md)。macOS 自动更新已实测通过（0.1.1 → 0.3.1），Windows 安装与升级仍待实机验收。

## 开发

技术栈为 **Rust + Tauri 2 + Vue 3 + TypeScript**。环境准备、构建、测试与提交约定都在开发文档里：

- [开发文档索引](dev-docs/README.md)
- [开发指南](dev-docs/DEVELOPMENT.md)
- [约定手册](dev-docs/CONVENTIONS.md)
- [需求时间线](dev-docs/need-do.md)

## 致谢

特别感谢 [**XJTUToolBox（仙交百宝箱）**](https://github.com/yan-xiaoo/XJTUToolBox)：一个基于 PyQt5 的优秀校园工具箱，覆盖课表、考勤、评教、成绩、思源学堂等场景。本项目的校园协议、接口行为与交互思路大量参考自它，没有它就没有今天的 XJTU Launcher。强烈推荐前往仓库 Star、关注并参与贡献：[yan-xiaoo/XJTUToolBox](https://github.com/yan-xiaoo/XJTUToolBox)。

XJTUToolBox 以 **GPL-3.0** 许可证发布；本项目按其公开行为独立用 Rust 重新实现，不复制其源码，也不将其作为运行时依赖。若未来直接复用其代码，将按其许可证要求处理。

同时感谢 [Tauri](https://tauri.app/)、[Vue](https://vuejs.org/)、[lucide](https://lucide.dev/)、[Noto Sans SC](https://fonts.google.com/noto/specimen/Noto+Sans+SC) 等开源项目。

## 许可证

本项目以 [MIT](LICENSE) 许可证开源，Copyright (c) 2026 haiziyao。

内嵌的 Noto Sans SC 字体按其 SIL Open Font License 1.1 分发，许可证文本见 `src/assets/fonts/OFL.txt`。

桌面宠物的四张精灵图来自社区 Codex Pet 生态，**不适用 MIT**，仅限非商业使用并保留作者署名；逐项来源与条款见 `src/assets/pets/NOTICE.md`。本项目免费、非商业，不售卖这些素材，也不与 DeepSeek、OpenAI、Anthropic 存在隶属关系。
