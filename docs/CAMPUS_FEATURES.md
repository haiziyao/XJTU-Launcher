# 校园功能与 Agent 接续（2026-10-19）

用户要求将 XJTUToolBox 的校园功能迁入本项目，特别是一键评教；可以插件化或放入功能市场，随后实现 Agent。范围仍包含全部校园功能，不能把本批几个接口当作全部完成。

## 产品安排与设计状态

沿用现有插件市场承载校园功能，不额外增加平级市场入口。校园功能以独立插件提供页面及可复用业务接口；之后 Agent 工具使用同一业务接口。跨页面的数据归属使用账号 ID 与会话代次。

四张设计稿位于 `design/campus-agent-v1/`。用户已回答“需要调整，我会说明”；它们尚未获批，未实现新前端页面。绘图脚本只是独立设计资料，不是应用页面。后续插件市场 `plugin-market-v2` 已独立获准并实现；此处 Agent 与评教页面仍需调整、确认后再落地。

## 参考范围

参考本地 `/Users/bilibili/Developer/HZY/References/XJTUToolBox`，提交 `e6a2d3b603ff7a803cc1d658a1fb5d52ca6b511b`。参考仓库含 GPL-3.0 许可证；本批通过阅读接口路径、请求参数和响应字段编写独立 Rust 适配代码，未把 Python 模块、评价模板或 UI 素材复制入仓库。后续需要直接复用上游代码时另行核对许可证及分发安排。

| 功能 | 参考入口 | 本批状态 |
| --- | --- | --- |
| 账号、WebVPN、课表 | `auth/`、`attendance/` | 既有实现，用户已验证登录及课表 |
| 本科成绩 | `jwxt/score.py` | Rust 查询、分页与结果归一化；未实机验收 |
| 本科评教 | `jwxt/judge.py` | Rust 问卷列表、预览、模板/自定义答案、逐份提交与取消；未实机验收 |
| 考试安排 | `jwxt/schedule.py` | Rust 查询适配；未实机验收 |
| 空闲教室 | `jwxt/empty_room.py`、`jwxt/util.py` | Rust 校区/楼宇选项、学生身份选择、按日期节次查询；未实机验收 |
| 全校课程 | `jwxt/school_course.py` | Rust 学期/课程名/校区筛选及分页；未实机验收 |
| 校历 | `jwxt/calendar.py` | Rust 图片链接解析，未知学年保留为空；未实机验收 |
| 研究生成绩、课表与评教 | `gmis/`、`gste/` | 待适配；不能套用本科接口 |
| 考勤统计与流水 | `attendance/attendance.py` | Rust 查询适配：`Query::AttendanceStreams` / `AttendanceRecords`，复用账号的考勤业务 token；未实机验收 |
| 思源学堂、作业、资料、回放 | `lms/lms.py`、`app/threads/LMS*` | 待适配 |
| 通知订阅与推送 | `app/threads/NoticeThread.py` | 待适配 |
| 校园卡余额与流水 | `card/campus_card.py` | Rust 查询适配：CAS Cookie 换 ncard 令牌，余额与流水分页；未实机验收 |
| 图书馆座位与预约 | `library/seats.py` | 待适配 |
| 体测成绩 | `fitness/score.py` | 待适配 |
| 场馆查询与预约 | `app/sessions/venue_session.py` | 待适配 |
| 学籍档案 | `app/sessions/hello_session.py` | 待适配 |

参考仓库有实现不代表学校当前环境必然可用；所有新服务的真实账号联调单独记录。

## 当前后端接口

2026-10-23 补充：考勤流水与课程考勤查询已接入 `school_query`（`attendanceStreams` / `attendanceRecords`），复用账号保存的本科/研究生考勤业务 token，按账号当前直连或 WebVPN 模式访问。校园卡查询（`card` / `cardTransactions`）用已保存的 CAS Cookie 向 ncard 换临时令牌，支持余额与流水。两者页面尚未实现，未实机验收。

`src-tauri/src/school/` 编译入 Rust，主窗口可调用：

- `school_catalog`：校园能力清单及后端进度，现已接入插件市场校园列表，尚无新业务页面的项目显示待接入；`pending` 项不是可用功能。
- `school_query`：查询请求使用封闭的 `Query` 枚举，不接受任意 URL。支持成绩、考试、学期、评教列表、教室选项、空闲教室、全校课程、校历、考勤、校园卡。
- `school_prepare_evaluations`：按当前账号重新读取开放问卷及真实选项，生成精确的待提交答案。模板等级 1–5 对应选项顺序；文字意见由用户提供，必填文字题为空时返回错误。支持逐题覆盖答案。预览不会发送评价。
- `school_submit_evaluations`：消费一次性预览 ID 后逐份提交；账号、会话代次与 15 分钟有效期必须匹配。返回每份 `submitted/failed/skipped/cancelled/unknown` 结果，并发送 `school-evaluation-progress` 事件。网络结果不明时不自动重发，停止余下提交，提示刷新学校状态。
- `school_cancel_evaluations`：丢弃尚未执行的预览，或在当前请求返回后停止后续提交。已经发送的请求不会宣称撤回成功。

预览仅保存在本次进程内。重启需要重新查询并预览。查询开始时固定账号快照，返回时检查会话代次，防止切换账号后混入数据。提交期间串行化账号切换；取消通道独立。已确认的提交结果不会因本地会话保存失败而变成可重试失败。

教务连接复用 `auth.json` 的 Cookie，按当前账号直连/WebVPN 模式进入教务首页并验证用户接口。新增白名单仅为 `jwxt.xjtu.edu.cn` 与 `dean.xjtu.edu.cn`。这批尚未把 CAS 登录状态机扩展成所有站点的自动密码重认证；TGC 或教务登录态失效会明确报错。不可宣称跨所有网站免登。

## 后续实现顺序

1. 按用户反馈修订页面参考稿，确认后接通现有插件管理及新的查询/评教页面。
2. 验证本科教务的会话复用与请求格式，补站点自动重认证，实际提交由用户在客户端操作验收。
3. 继续迁入研究生教务、思源学堂和校园生活模块，逐项标记实现/联调/可用状态；考勤后台已适配，待实机验收。
4. 实现完整 Agent：模型流式响应、工具循环、取消和重试、持久对话、文件与命令工具、校园插件工具；复用现有多厂商模型配置。
5. Agent 扩展：MCP（stdio 与 Streamable HTTP）、Skills 发现与执行、长期记忆读写/检索/删除和跨对话恢复。Agent 数据继续放 `~/.xjtu-launcher/`，账号和密钥仍放 `auth.json`。

Agent 本批仍未实现，不能把设计稿中的对话和工具成功状态当成可用能力。页面引用的所有课程、评价和对话均是设计示例。

## 初始后台批次验证（历史）

`cargo test`：42 项通过，2 项联网探测默认忽略；`cargo clippy --all-targets -- -D warnings` 通过。新增测试覆盖学期和日期校验、成绩缺失值保留、校历来源筛选、问卷预览与提交数据一致、必填文字题与未知题型、账号/会话/有效期绑定、错误响应不可伪装为空列表。没有提交真实评价，没有使用用户 API Key 调用模型。本批没有新增前端实现，也没有替换正在运行的已验收 App。
