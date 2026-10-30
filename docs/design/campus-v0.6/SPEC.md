# v0.6.0 校园原生化：需求、功能与接口规格

更新：2026-10-30
状态：**范围已确认（2026-10-30），视觉未获准**。用户确认了信息架构、P0 范围和第 6 节的 6 项决定；参考图尚未出、未获准，按 [约定手册](../../../dev-docs/CONVENTIONS.md#2-设计与前端)，参考图获批前不改前端。排版见 [LAYOUT.md](LAYOUT.md)。
依据：[页面与接口清单](INVENTORY.md)、[功能颗粒清单](FEATURES.md)、当前代码、参考项目 XJTUToolBox 的协议记录（只借鉴协议，独立实现）。

## 0. 目标与边界

目标：

1. 校园数据全部在应用内展示。“打开校园网页”只做兜底，不再是任何功能的主路径。
2. 从“工具列表 + 单页查询”改为按任务组织：先告诉用户要做什么（待交作业、下一场考试、待评教），再给明细。
3. 每个领域除了实体列表，还要提供用户关心的**过程状态、指标和筛选维度**。第 1 节是推导依据。

边界：

- 当前只适配本科教务；研究生系统（参考项目里的 gmis / gste）不在本版范围。
- 不绕过学校的任何校验。验证码由用户本人完成，不做自动识别；到馆签到等校验以学校系统为准。
- 所有写操作统一“预览 → 用户明确操作 → 只提交一次 → 结果不明不重试”（第 4.5 节）。
- 本地计算的指标（绩点、均分、出勤率、消费统计）在界面上标注“按本页数据计算，以学校系统为准”。
- 优先级：P0 = v0.6 必做；P1 = v0.6 应做，可随版本推进；P2 = 依赖未适配协议或新依赖，先探测再定。最终范围由用户确认。

## 1. 用户视角

> 本节是**推测**：按校历、课程周期和常见校园场景推导，没有用户调研或埋点数据支撑。它的作用是找出“数据里有、但页面没给”以及“数据里没有、但用户一定会想要”的东西。

### 1.1 使用节奏

| 时间            | 用户在做什么             | 打开应用最想看到                 |
| --------------- | ------------------------ | -------------------------------- |
| 每天早上 / 课间 | 看今天的课，下一节在哪   | 下一节课倒计时、地点、今日课程   |
| 每天晚上        | 看有没有要交的作业       | 今天 / 明天截止的作业、未交数量  |
| 每周            | 下载资料、看回放、看通知 | 新资料、新通知、本周到期         |
| 想自习时        | 找教室或座位             | 现在有空的教室、各区域空位       |
| 运动前          | 约场地                   | 哪天哪个时段还有场               |
| 出分期间        | 反复刷成绩               | 新出的成绩、加权均分 / 绩点变化  |
| 考试周          | 确认时间、地点、座位     | 下一场考试倒计时、同一天多场提醒 |
| 学期末          | 评教                     | 待评数量、截止时间、一次评完     |
| 消费后          | 看余额                   | 余额、本月花了多少               |
| 填表 / 办事     | 查学号、班级、辅导员电话 | 一键复制                         |

### 1.2 各领域的过程、指标、维度

#### 作业（思源学堂）

- 过程：老师布置 → 发现新作业 → 看要求和附件 → 完成 → 提交（可能先存草稿，可能交多次）→ 等批改 → 看分数和评语 → 需要时重交。
- 指标：未交数、最近截止倒计时、本周到期数、逾期数、已交 / 总数、我的得分与班级平均 / 最高的对比、待批改数。
- 维度：课程、截止区间（已逾期 / 今天 / 明天 / 本周 / 以后 / 无截止）、状态、学期、个人 / 小组。
- 推测需求：新作业和作业变更提示；截止前提醒；提交凭证（时间、文件名、大小）；重交前能看到上次交了什么。

#### 学习资料

- 过程：课前或复习时找资料 → 预览 → 下载或批量下载 → 本地整理。
- 指标：新资料数、文件数和总大小。
- 维度：课程、文件类型（文档 / 课件 / 视频 / 压缩包）、更新时间、是否已下载。
- 推测需求：按课程直接列出全部文件，不必一个个点开活动；按文件名搜索；已下载标记；批量下载时按课程建子目录。

#### 课程回放

- 过程：课后或复习 → 找某一次课 → 播放 → 下次接着看。
- 指标：时长、是否看过。
- 维度：课程、日期、周次。
- 推测需求：应用内播放、续播、倍速（依赖播放协议，见 4.9）。

#### 成绩

- 过程：出分期间反复查 → 看新出的单科 → 算均分和绩点 → 和以前的学期比 → 必要时找教务复核。
- 指标：加权均分、加权绩点、学期绩点、已修学分、课程数、不及格门数 / 学分、新出成绩数、各课程性质的学分。
- 维度：学期 / 学年、课程性质、是否及格、分数段。
- 推测需求：新出成绩高亮（和上次缓存比）；学期趋势；排名和毕业学分要求没有数据来源，不做。

#### 考试

- 过程：考试安排公布 → 确认时间、地点、座号 → 规划复习 → 考前再确认。
- 指标：距下一场天数、剩余场次、本周场次、同一天的场次。
- 维度：日期、学期、校区、课程。
- 推测需求：同日多场或时间冲突提示；导出到系统日历；考完自动折叠。

#### 评教

- 过程：学校开放 → 看待评列表 → 填写 → 提交 → 确认全部完成。
- 指标：待评数、已评数、截止时间。
- 维度：学期、课程、教师、问卷类型。
- 推测需求：每位老师单独写评语；进度一眼可见；截止提醒。

#### 课表

- 过程：每天看今天和下一节 → 临时调课时手动修改。
- 指标：今日节数、下一节倒计时、本周课时。
- 维度：周次、星期、节次。
- 推测需求：钟点时间（课表接口已返回作息）；上课提醒；课间空档直接查空教室；导出到系统日历。

#### 空闲教室

- 过程：想自习或小组讨论 → 选时间 → 选附近的楼 → 看容量。
- 指标：空闲教室数、容量。
- 维度：校区、楼宇、时段、容量、类型。
- 推测需求：“现在有空”一键查；记住常用楼；按容量筛选和排序。

#### 图书馆

- 过程：出发前看空位 → 预约 → 到馆签到 → 中途离开 / 返回 → 离馆。
- 指标：各区域空余 / 总数、空余率、我的预约状态。
- 维度：校区、楼层、区域。
- 推测需求：区域对比，不必一个个点；收藏常用区域；预约后的签到时限提醒（时限待核实）。

#### 场馆

- 过程：约好人 → 看哪天哪个时段有场 → 预订（有验证码）→ 到场 → 需要时取消。
- 指标：可订时段数、价格、余量、我的订单状态。
- 维度：运动类型、场馆、日期、时间段。
- 推测需求：时间 × 场地矩阵一眼看全；提前预约开放时间提示（`advanceDays`）。

#### 校园卡

- 过程：查余额 → 看最近消费 → 看这个月花了多少 → 核对异常扣款。
- 指标：余额、待入账、本期支出、收入、笔数、日均、最大一笔、商户排行。
- 维度：时间（日 / 周 / 月 / 自定义）、商户、交易类型、收支。
- 推测需求：低余额提醒（本地阈值）；按商户查流水。

#### 考勤

- 过程：被提醒或评奖前 → 看出勤 → 找异常记录 → 线下申诉。
- 指标：到课率、准时率、迟到 / 缺勤 / 请假次数。
- 维度：课程、周次、日期、状态。
- 推测需求：只看异常；按课程汇总。

#### 体测

- 过程：测完查成绩 → 看单项 → 和及格线比 → 准备补测。
- 指标：总分、等级、单项分。
- 维度：学年、项目。
- 推测需求：历年对比；单项标准需要可靠数据来源，未定。

#### 通知

- 过程：每天扫一眼 → 看感兴趣的栏目 → 读正文 → 下载附件 → 记下截止时间。
- 指标：未读数、今日新增。
- 维度：栏目（7 个）、分类、日期、已读 / 未读、收藏。
- 推测需求：多栏目合并时间线；搜索；关键词关注（参考项目有过滤规则）；收藏。

#### 学籍

- 过程：填表时查学号、班级、辅导员电话。
- 推测需求：一键复制；生日、卡号等敏感字段默认打码。

### 1.3 横向需求

- **统一待办**：作业、评教、考试；之后接入图书馆签到和场馆订单。
- **新内容提示**：新作业、新资料、新成绩、新通知，都和上次缓存比较得出。
- **数据新鲜度**：每个分区显示“更新于 …”，可手动刷新。
- **搜索**：每个列表页都有就地搜索；跨课程资料搜索只搜已缓存的内容。
- **导出**：考试、课表、作业截止导出 `.ics`；成绩和流水导出 CSV（P2）。
- **提醒**：应用内提醒（主页、侧栏角标）加系统通知（Tauri 通知插件，已确认，见 X-08）。

## 2. 信息架构

页面结构和线框见 [LAYOUT.md](LAYOUT.md)。核心变化：

- **主页**保持 DIY，新增“下一节课”“待办摘要”“指标条”组件；“校园快捷入口”改为“常用功能”，只放应用内深链。
- **畅行校园**从 13 个工具列表改为带分区导航的工作区：待办、通知、学习、学业、生活。
- **学籍**移到个人页。

### 2.1 外链替换

| 现有外链               | 替换为                                            | 优先级 |
| ---------------------- | ------------------------------------------------- | ------ |
| 主页“教务系统”         | 常用功能：成绩 / 考试 / 评教等应用内分区          | P0     |
| 主页“思源学堂”         | 常用功能：待办 / 思源学堂分区                     | P0     |
| 主页“图书馆”           | 常用功能：图书馆分区                              | P0     |
| 主页“一网通办”         | 移除，不保留兜底入口（2026-10-30 决定）           | P0     |
| 通知正文               | 应用内阅读器 `NoticeDetail`                       | P0     |
| 校历图片               | 应用内图片查看 `campus_image`                     | P0     |
| LMS 附件“打开”         | 应用内预览 `lms_preview_file`；不支持的格式只下载 | P0     |
| LMS 回放               | 应用内播放 `LmsReplayVideos`；取不到时兜底        | P1     |
| LMS 直播 / 讨论 / 问卷 | 显示基本信息 + 兜底                               | P2     |

兜底入口统一放在分区标题的“更多（⋯）”菜单，文案“在校园网页中打开”，不做成主按钮。

## 3. 功能定义

编号用于后续排期和验收追踪。数据来源中带 `新` 的接口见第 4 节。

### 3.1 主页（HOME）

| ID      | 功能         | 说明 / 规则                                                                                  | 数据来源                             | 优先级 |
| ------- | ------------ | -------------------------------------------------------------------------------------------- | ------------------------------------ | ------ |
| HOME-01 | 下一节课     | 当前或下一节：课程、地点、开始时间、倒计时；钟点取学校返回的 `periods`                       | 课表 + `periods`                     | P0     |
| HOME-02 | 今日课程     | 沿用现有卡片；已结束的课置灰                                                                 | 课表                                 | P0     |
| HOME-03 | 待办摘要     | 最近 5 条（作业 / 评教 / 考试），点击进入对应详情                                            | 新 `LmsTodos` + 评教 + 考试          | P0     |
| HOME-04 | 指标条       | 校园卡余额、本周待交作业、距下一场考试、未读通知；每块独立加载，点击跳转；单块失败不影响其他 | 各领域缓存                           | P0     |
| HOME-05 | 通知         | 可选栏目（默认教务处），点击进应用内阅读器                                                   | `campus_notices` + 新 `NoticeDetail` | P0     |
| HOME-06 | 常用功能     | 替代四个外链；内容来自 `campusPinned`，可编辑，只能放应用内分区                              | 配置                                 | P0     |
| HOME-07 | 元素开关排序 | 沿用 `homeSections` / `widgets`；新增 `next-class`、`todo`、`stats`                          | 配置                                 | P0     |

### 3.2 待办（TODO）

| ID      | 功能       | 说明 / 规则                                                      | 数据来源                             | 优先级 |
| ------- | ---------- | ---------------------------------------------------------------- | ------------------------------------ | ------ |
| TODO-01 | 统一待办   | 作业（未交 / 可重交 / 逾期仍可交）、评教（待评）、考试（未开始） | 新 `LmsTodos`、评教、考试            | P0     |
| TODO-02 | 时间分组   | 已逾期 / 今天 / 明天 / 本周 / 以后 / 无截止，组内按截止时间升序  | 前端 `todoBuckets`                   | P0     |
| TODO-03 | 筛选       | 类型、课程、状态；默认隐藏已完成，可切换“显示已完成”             | 前端                                 | P0     |
| TODO-04 | 部分失败   | 某门课读取失败时在列表顶部列出，可单独重试                       | `LmsTodos.failures`                  | P0     |
| TODO-05 | 新增标记   | 新出现或 `updatedAt` 变化的项标“新”，查看后清除                  | 账号缓存快照                         | P1     |
| TODO-06 | 预约类待办 | 图书馆待签到、场馆未开始订单                                     | 新 `LibraryMyBooking`、`VenueOrders` | P2     |

### 3.3 思源学堂（LMS）

| ID     | 功能       | 说明 / 规则                                                                                                       | 数据来源                        | 优先级 |
| ------ | ---------- | ----------------------------------------------------------------------------------------------------------------- | ------------------------------- | ------ |
| LMS-01 | 课程列表   | 默认本学期，可切“全部学期”；搜索课程名 / 代码 / 教师；每门课显示未交作业数和新资料数                              | `LmsCourses` + `LmsTodos`       | P0     |
| LMS-02 | 课程主页   | 课程名、教师、学分、起止日期；分类标签带数量                                                                      | `LmsActivities`                 | P0     |
| LMS-03 | 作业列表   | 状态（未交 / 已交 / 已截止未交 / 逾期仍可交）、截止倒计时、小组标记；默认按截止升序                               | `LmsTodos` / `LmsActivity`      | P0     |
| LMS-04 | 作业详情   | 要求、截止、是否已截止、可否重交、已交 / 上限、班级统计、附件                                                     | `LmsActivity`（补字段）         | P0     |
| LMS-05 | 我的提交   | 每次提交的时间、文字、文件、得分、老师评语、批改附件、是否最新、能否撤回                                          | `LmsSubmissions`                | P0     |
| LMS-06 | 提交作业   | 系统对话框多选或拖入文件（≤10 个，单个 100 MB，总计 200 MB，扩展名白名单）+ 文字；草稿 / 正式；预览 → 确认 → 结果 | 新写操作 `LmsHomework`          | P0     |
| LMS-07 | 附件预览   | 图片、文本、PDF 在应用内看；其他格式只下载                                                                        | 新 `lms_preview_file`           | P0     |
| LMS-08 | 附件下载   | 单个保存，记住上次目录；已下载标记（本地记录）                                                                    | `lms_save_file`                 | P0     |
| LMS-09 | 批量下载   | 勾选多个文件下载到一个目录，可按课程 / 活动建子目录；显示进度                                                     | 新 `lms_save_files`             | P1     |
| LMS-10 | 课程资料   | 资料标签直接列出该课全部文件（展开资料活动），按文件名搜索、按类型筛选                                            | 新 `LmsCourseFiles`             | P0     |
| LMS-11 | 跨课程搜索 | 在已缓存的课程文件中按文件名搜索                                                                                  | 前端缓存                        | P1     |
| LMS-12 | 回放       | 按日期列出、显示时长；应用内播放，两路画面可切换；链接过期时重新获取；不可用时兜底                                | 新 `LmsReplayVideos`            | P1     |
| LMS-13 | 其他活动   | 直播、讨论、问卷、网页链接：显示基本信息 + 兜底                                                                   | `LmsActivity`                   | P2     |
| LMS-14 | 撤回提交   | 仅 `canRetract` 时显示，走写操作流程                                                                              | 新写操作 `LmsRetract`（待探测） | P2     |

### 3.4 成绩（GRD）

| ID     | 功能     | 说明 / 规则                                                                             | 数据来源            | 优先级 |
| ------ | -------- | --------------------------------------------------------------------------------------- | ------------------- | ------ |
| GRD-01 | 指标     | 加权均分、加权绩点、已修学分、课程数、不及格门数 / 学分、未计入均分门数；随筛选范围变化 | 前端 `gradeStats`   | P0     |
| GRD-02 | 筛选排序 | 学期（全部 / 各学期）、课程性质、搜索；按学期、成绩、学分、绩点排序                     | `Grades`            | P0     |
| GRD-03 | 成绩表   | 课程、性质、学分、成绩、绩点、学期、是否及格                                            | `Grades`            | P0     |
| GRD-04 | 新出成绩 | 和上次缓存比，新增行标“新”，顶部提示“新出 N 门”                                         | 账号缓存快照        | P0     |
| GRD-05 | 学期趋势 | 每学期加权绩点折线（少于 2 个学期时隐藏）                                               | 前端                | P1     |
| GRD-06 | 学分构成 | 按课程性质汇总已修学分                                                                  | 前端                | P1     |
| GRD-07 | 导出 CSV | 导出当前筛选结果                                                                        | 新 `save_text_file` | P2     |
| GRD-08 | 成绩构成 | 展开单科看平时 / 期末 / 实验 / 其他分项和占比；标注初修 / 重修                          | `Grades`（补映射）  | P1     |

### 3.5 考试（EXM）

| ID     | 功能         | 说明 / 规则                                | 数据来源             | 优先级 |
| ------ | ------------ | ------------------------------------------ | -------------------- | ------ |
| EXM-01 | 下一场考试   | 课程、日期、时间、校区、考场、座号、倒计时 | `Exams`              | P0     |
| EXM-02 | 日程列表     | 按日期分组；同一天多场时提示；已结束的折叠 | `Exams`              | P0     |
| EXM-03 | 学期切换     | 后端已支持 `term`                          | `Exams { term }`     | P1     |
| EXM-04 | 导出日历     | 导出 `.ics`                                | 新 `export_calendar` | P1     |
| EXM-05 | 课表冲突提示 | 考试时间和课表重叠时标记                   | 前端                 | P2     |

### 3.6 评教（EVA）

| ID     | 功能         | 说明 / 规则                                    | 数据来源                         | 优先级 |
| ------ | ------------ | ---------------------------------------------- | -------------------------------- | ------ |
| EVA-01 | 进度         | 已评 / 待评 / 最早截止                         | `Evaluations` 两次查询           | P0     |
| EVA-02 | 待评与评语   | 沿用现有流程；新增“每位老师单独评语”           | `Prepare.comments`（后端已有）   | P0     |
| EVA-03 | 已评列表     | 只读                                           | `Evaluations { finished: true }` | P0     |
| EVA-04 | 预览提交结果 | 保留现有三条命令和逐份结果，不迁移到通用写模型 | 现有命令                         | P0     |

评教页视觉仍**未获准**，必须单独出图确认。

### 3.7 课表（TT）

| ID    | 功能       | 说明 / 规则                          | 数据来源             | 优先级 |
| ----- | ---------- | ------------------------------------ | -------------------- | ------ |
| TT-01 | 钟点时间   | 节次映射到钟点，网格左侧显示         | 课表 `periods`       | P1     |
| TT-02 | 当前时间线 | 当前时刻横线、正在上的课高亮         | 前端                 | P1     |
| TT-03 | 导出日历   | 导出本学期课程 `.ics`                | 新 `export_calendar` | P1     |
| TT-04 | 空档查教室 | 点空白格跳到空闲教室，带入日期和节次 | 前端深链             | P1     |

### 3.8 空闲教室（ROOM）

| ID      | 功能     | 说明 / 规则                                                | 数据来源              | 优先级 |
| ------- | -------- | ---------------------------------------------------------- | --------------------- | ------ |
| ROOM-01 | 快捷时段 | 现在、上午、下午、晚上、自定义；“现在”从当前节到本大节结束 | 前端 + 课表 `periods` | P0     |
| ROOM-02 | 记住选择 | 记住上次的校区和楼宇；收藏常用楼宇                         | 配置                  | P0     |
| ROOM-03 | 结果筛选 | 容量下限、类型；按容量排序                                 | 前端                  | P0     |

### 3.9 全校课程（CRS）

| ID     | 功能     | 说明 / 规则                                | 数据来源  | 优先级 |
| ------ | -------- | ------------------------------------------ | --------- | ------ |
| CRS-01 | 查询     | 学期、课程名、校区（后端已支持，页面补上） | `Courses` | P1     |
| CRS-02 | 完整结果 | 取消 200 条截断，改为分页或虚拟滚动        | `Courses` | P1     |
| CRS-03 | 本地筛选 | 教师、院系、学分                           | 前端      | P1     |
| CRS-04 | 收藏     | 本地收藏课程，用于选课比较                 | 配置      | P2     |

### 3.10 校历（CAL）

| ID     | 功能     | 说明 / 规则              | 数据来源          | 优先级 |
| ------ | -------- | ------------------------ | ----------------- | ------ |
| CAL-01 | 当前周   | 当前学期、第 N 教学周    | 课表 `startDate`  | P0     |
| CAL-02 | 校历图片 | 应用内查看、缩放、另存为 | 新 `campus_image` | P0     |

学校只提供校历图片，没有结构化的假期日期，所以不做假期倒计时。

### 3.11 校园卡（CARD）

| ID      | 功能       | 说明 / 规则                                                                       | 数据来源                | 优先级 |
| ------- | ---------- | --------------------------------------------------------------------------------- | ----------------------- | ------ |
| CARD-01 | 余额状态   | 余额、待入账（`pending`，已返回未显示）、卡状态、有效期                           | `Card`                  | P0     |
| CARD-02 | 流水       | 收支筛选、商户搜索、区间选择、分页                                                | `CardTransactions`      | P0     |
| CARD-03 | 区间指标   | 支出、收入、笔数、日均、最大一笔；区间：本周 / 本月 / 近 30 天 / 自定义（≤92 天） | 新 `CardSummary`        | P1     |
| CARD-04 | 每日支出图 | 柱状图，单一色相                                                                  | `CardSummary.days`      | P1     |
| CARD-05 | 商户排行   | 前 8 名 + 其他                                                                    | `CardSummary.merchants` | P1     |
| CARD-06 | 低余额提示 | 本地阈值，默认关闭                                                                | 配置                    | P2     |

### 3.12 考勤（ATT）

| ID     | 功能       | 说明 / 规则                                                          | 数据来源               | 优先级 |
| ------ | ---------- | -------------------------------------------------------------------- | ---------------------- | ------ |
| ATT-01 | 课程考勤   | 日期、节次、周次、地点、教师、状态                                   | `AttendanceRecords`    | P1     |
| ATT-02 | 指标       | 到课率、准时率、迟到 / 缺勤 / 请假次数；区间：本学期 / 本月 / 自定义 | 前端 `attendanceStats` | P1     |
| ATT-03 | 按课程汇总 | 每门课各状态次数                                                     | 前端                   | P1     |
| ATT-04 | 只看异常   | 迟到、缺勤                                                           | 前端                   | P1     |
| ATT-05 | 刷卡流水   | 沿用现有；补日期范围                                                 | `AttendanceStreams`    | P1     |

考勤记录原始响应有 `courseName`、`courseCode`（见 4.9），后端补映射即可按课程汇总；`appealEnabled` 为真时显示“可申诉”（申诉本身到学校系统办理）。

### 3.13 图书馆（LIB）

| ID     | 功能           | 说明 / 规则                                         | 数据来源                                 | 优先级 |
| ------ | -------------- | --------------------------------------------------- | ---------------------------------------- | ------ |
| LIB-01 | 区域总览       | 当前楼层各区域空余 / 总数，按空余率排序             | `LibrarySeats.stats`（已返回）           | P0     |
| LIB-02 | 座位图         | 沿用现有；显示刷新时间                              | `LibrarySeats`                           | P0     |
| LIB-03 | 记住与收藏     | 记住校区、楼层、区域；收藏常用区域                  | 配置                                     | P1     |
| LIB-04 | 我的预约       | 座位、时间、状态、可执行的动作                      | 新 `LibraryMyBooking`                    | P2     |
| LIB-05 | 预约与到馆动作 | 预约 / 换座 / 取消 / 入馆签到 / 中途离开 / 中途返回 | 新写操作 `LibraryBook` / `LibraryAction` | P2     |

### 3.14 场馆（VEN）

| ID     | 功能     | 说明 / 规则                                                     | 数据来源               | 优先级 |
| ------ | -------- | --------------------------------------------------------------- | ---------------------- | ------ |
| VEN-01 | 场馆列表 | 按运动类型筛选                                                  | `VenueList`            | P1     |
| VEN-02 | 时段矩阵 | 时间 × 场地，格子显示可订 / 已满 / 不可订和价格                 | `VenueSlots`           | P1     |
| VEN-03 | 我的订单 | 逐条明细（已返回，页面只显示一行）                              | `VenueOrders`          | P1     |
| VEN-04 | 预订     | 选时段 → 预览 → 用户本人在应用内完成验证码 → 提交（方式已确认） | 新写操作 `VenueBook`   | P2     |
| VEN-05 | 取消订单 | 写操作                                                          | 新写操作 `VenueCancel` | P2     |

### 3.15 体测（FIT）

| ID     | 功能     | 说明 / 规则  | 数据来源                   | 优先级 |
| ------ | -------- | ------------ | -------------------------- | ------ |
| FIT-01 | 学年成绩 | 沿用现有     | `Fitness` / `FitnessScore` | P1     |
| FIT-02 | 历年对比 | 逐年惰性查询 | `FitnessScore`             | P2     |

### 3.16 通知（NTC）

| ID     | 功能       | 说明 / 规则                                | 数据来源              | 优先级 |
| ------ | ---------- | ------------------------------------------ | --------------------- | ------ |
| NTC-01 | 通知中心   | 7 个栏目；单栏目，或“全部”合并后按日期排序 | `campus_notices`      | P0     |
| NTC-02 | 阅读器     | 正文、图片、附件；原文链接放兜底菜单       | 新 `NoticeDetail`     | P0     |
| NTC-03 | 已读未读   | 本地记录，按账号隔离；显示未读数           | 本地                  | P0     |
| NTC-04 | 搜索       | 在已取回范围内搜标题                       | 前端                  | P0     |
| NTC-05 | 附件下载   | 原生保存                                   | 新 `campus_save_file` | P0     |
| NTC-06 | 收藏       | 本地                                       | 本地                  | P1     |
| NTC-07 | 关键词关注 | 命中的通知标记并计入主页                   | 本地                  | P2     |

### 3.17 个人页（ME）

| ID    | 功能     | 说明 / 规则                              | 数据来源  | 优先级 |
| ----- | -------- | ---------------------------------------- | --------- | ------ |
| ME-01 | 学籍信息 | 从畅行校园移到个人页；学号、电话一键复制 | `Profile` | P1     |
| ME-02 | 敏感打码 | 生日、卡号、电话默认打码，点击显示       | 前端      | P1     |

### 3.18 横向（X）

| ID   | 功能     | 说明 / 规则                                                                                                                         | 优先级 |
| ---- | -------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------ |
| X-01 | 刷新     | 每个分区标题统一显示“更新于 …”和刷新按钮                                                                                            | P0     |
| X-02 | 缓存优先 | 先显示缓存再后台刷新（沿用 `applyCached`）；缓存超过 24 小时标“可能已过期”                                                          | P0     |
| X-03 | 错误分类 | 登录失效 / 网络 / 仅校园网 / WebVPN 下不可用 / 学校服务异常 / 部分失败，文案和操作见 LAYOUT §8                                      | P0     |
| X-04 | 账号切换 | 关闭阅读器、预览、播放器和写操作预览；丢弃在途请求；清空选择                                                                        | P0     |
| X-05 | 键盘     | 列表 ↑↓ 选择、Enter 打开、Esc 返回、⌘/Ctrl+F 聚焦搜索                                                                               | P0     |
| X-06 | 兜底外链 | 统一放在“更多（⋯）”菜单                                                                                                             | P0     |
| X-07 | 操作记录 | 写操作结果记入本地（时间、类型、对象、结果），不含文件内容和 Cookie                                                                 | P1     |
| X-08 | 系统提醒 | 作业截止 / 考试 / 上课提醒，使用系统通知（引入 Tauri 通知插件，已确认）；每类可单独关闭，默认只开截止提醒；上课时间取课表 `periods` | P1     |

## 4. 接口定义

### 4.1 约定

- 新查询都加到 `school::Query`（`src-tauri/src/school/queries.rs`），前端通过 `bridge.schoolQuery`、`siteQuery`、`directQuery` 调用，沿用账号代次检查和按账号缓存。
- 新命令都要 `main_only(&window)`，校园功能还要 `require_school_feature`。
- JSON 字段用 camelCase。学校的时间字段保持原样字符串；需要倒计时的地方额外提供 `dueAt`（Unix 毫秒，按 Asia/Shanghai 解析，解析失败为 `null`）。
- 聚合查询返回 `partial` 和 `failures`，不因为单项失败整体报错。
- 文件预览和下载的临时文件放在按账号隔离的缓存目录，账号删除时一起清理。
- 新查询同步作为只读动作加入 Agent / MCP 工具；写操作和页面共用第 4.5 节的同一套实现。

### 4.2 现有类型补充（`src/types.ts`）

```ts
// 活动列表补字段（2026-10-30 实测原始响应已有）。列表不含 user_submit_count。
interface LmsActivity {
  // ...现有字段
  closed: boolean; // is_closed
  inProgress: boolean; // is_in_progress
  submitTimes: number | null;
  updatedAt: string;
}

// 后端 activity_detail() 已返回、前端尚未声明的字段。
interface LmsActivityDetail {
  // ...现有字段
  closed: boolean;
  resubmitOpen: boolean;
  allowRetract: boolean;
  mode: string;
}

// 对应 lms.rs 的 submissions()；后端已有，前端尚未声明。
interface LmsSubmission {
  id: number;
  status: string;
  score: number | null;
  rubricScore: number | null;
  submittedAt: string;
  updatedAt: string;
  createdAt: string;
  latest: boolean;
  resubmitted: boolean;
  canRetract: boolean;
  mode: string;
  comment: string;
  instructorComment: string;
  files: LmsUpload[];
  correction: {
    score: number | null;
    instructorScore: number | null;
    comment: string;
    updatedAt: string;
    files: LmsUpload[];
  } | null;
}
interface LmsSubmissions {
  activity: number;
  title: string;
  courseId: number | null;
  end: string;
  closed: boolean;
  submitByGroup: boolean;
  submissions: LmsSubmission[];
}

// 对应 attendance.rs 的 records()；course / courseCode / appealEnabled 为新增映射（原始字段已确认）。
// 原始响应里的班主任、辅导员、学院等个人字段不映射。
type AttendanceStatus =
  'pending' | 'normal' | 'late' | 'absent' | 'leave' | 'notRequired' | 'unknown';
interface AttendanceRecord {
  id: string;
  date: string;
  start: number;
  end: number;
  week: number | null;
  place: string;
  teacher: string;
  course: string;
  courseCode: string;
  appealEnabled: boolean;
  status: AttendanceStatus;
}
interface AttendanceRecords {
  page: number;
  total: number;
  rows: AttendanceRecord[];
}
```

`CardInfo.pending` 已有声明，页面直接使用即可。

### 4.3 新增查询

```rust
pub enum Query {
    // ...现有变体
    LmsTodos,                                  // 本学期作业待办聚合
    LmsCourseFiles { course: u64 },            // 某门课的全部文件
    LmsReplayVideos { activity: u64 },         // 回放视频直链
    NoticeDetail { url: String },              // 通知正文
    CardSummary { start: String, end: String },// 区间消费统计
    LibraryMyBooking,                          // 我的座位预约（P2）
}
```

#### `LmsTodos`

```ts
type LmsTodoState = 'todo' | 'overdue' | 'submitted' | 'missed';
interface LmsTodo {
  activityId: number;
  courseId: number;
  courseName: string;
  title: string;
  end: string;
  dueAt: number | null;
  group: boolean;
  submitCount: number | null;
  submitTimes: number | null;
  resubmitOpen: boolean;
  state: LmsTodoState;
  updatedAt: string;
}
interface LmsTodos {
  term: string;
  items: LmsTodo[];
  partial: boolean;
  failures: { courseId: number; courseName: string; message: string }[];
}
```

规则：

- 数据源用学校的官方待办接口 `GET /api/todos`：一次请求返回全部课程里未提交、未截止的作业（实测 11 条，逐条核对详情均为“未交、未关闭”）。不再逐课程、逐作业扫描（原方案实测需要 1 + 43 + 75 = 119 次请求）。
- `items` 只放待办，`state` 固定为 `todo`；截止已过但仍在列表里的标 `overdue`。`submitted`、`missed` 只出现在课程内的作业列表（LMS-03），不进待办。
- 课程内作业列表的状态推导（列表字段 + 待办集合，必要时再取详情）：
  - 在待办集合里 → 未交
  - 不在待办集合、未关闭 → 已交（推断；进入详情时用 `user_submit_count` 核实）
  - 已关闭 → 进入详情后按 `user_submit_count` 区分“已交”和“已截止未交”；列表先显示“已截止”
  - `is_resubmit_open` 时显示“可重交”
  - “已批改”需要提交记录，列表不判断，在详情页显示。
- `/api/todos` 只返回作业，评教和考试由各自接口补齐。
- 按账号缓存 5 分钟，手动刷新时绕过。

#### `LmsCourseFiles`

```ts
interface LmsCourseFile extends LmsUpload {
  activityId: number;
  activityTitle: string;
  activityType: string;
  updatedAt: string;
}
interface LmsCourseFiles {
  course: number;
  files: LmsCourseFile[];
  partial: boolean;
  failures: { activityId: number; message: string }[];
}
```

规则：展开该课 `material` 和 `homework` 活动的附件；并发上限 4；按账号缓存 30 分钟。

#### `LmsReplayVideos`

```ts
interface LmsReplayVideos {
  activity: number;
  videos: {
    id: number;
    label: string; // 例如教师画面 / 屏幕画面
    size: number;
    mute: boolean;
    bestAudio: boolean;
    url: string; // 预签名直链，会过期
  }[];
  fetchedAt: number;
}
```

规则（2026-10-30 实测）：

- 流程：`/api/lessons/{id}/player-url` 取 token → `rms-v5.xjtu.edu.cn/api/v1/auth/embed-token` 换 RMS 令牌 → `/api/embed/lesson-activities/captures/{replay_code}` 取 `lesson_videos`。令牌只在后端使用，不返回前端。
- `url` 取 `download_url`：`review-class.xjtu.edu.cn` 的 S3 预签名地址，不需要 Cookie，支持 Range，可以直接交给 `<video>`。主机名必须严格等于 `review-class.xjtu.edu.cn` 且为 HTTPS。
- 预签名地址会过期：每次点播放时重新获取，不缓存；播放中途失效时提示“回放链接已过期”，点击重新获取并从原进度继续。
- 每节课一般有 2 路画面；默认播放有声的一路（`mute=false`，或 `bestAudio=true`），另一路可切换。
- CSP 增加 `media-src https://review-class.xjtu.edu.cn`（用户已同意）。取不到视频时继续使用校园窗口兜底。
- 下载回放（P2）复用同一直链，走 `lms_save_file` 的流式下载与 2 GB 上限。

#### `NoticeDetail`

```ts
interface NoticeDetail {
  url: string;
  title: string;
  date: string;
  source: string;
  html: string;
  attachments: { name: string; url: string; size: number | null }[];
  truncated: boolean;
}
```

规则：

- URL 必须是 HTTPS，主机名在 7 个栏目的白名单内（`dean`、`xsc`、`gs`、`nic` 的 `.xjtu.edu.cn`）。
- 页面上限 2 MB（沿用 `MAX_PAGE_BYTES`）。后端提取正文容器，只保留白名单标签，去掉脚本、样式和事件属性。
- 同域图片最多 10 张、单张不超过 2 MB，内联为 data URL（CSP 已允许 `data:`）。超出的图片省略，并标记 `truncated`。
- 前端再用 DOMPurify 清洗一次。正文里的链接点击时走 `openUrl` 兜底，并提示“将打开校园网页”。

#### `CardSummary`

```ts
interface CardSummary {
  start: string;
  end: string;
  spend: number; // 分
  income: number; // 分
  count: number;
  largest: CardTransaction | null;
  days: { date: string; spend: number; income: number }[];
  merchants: { name: string; spend: number; count: number }[];
  partial: boolean;
}
```

规则：区间不超过 92 天；后端最多翻 20 页（600 条），超出时 `partial = true`；商户取前 8 名，其余合并为“其他”。金额单位为分，`amount < 0` 记为支出。

#### `LibraryMyBooking`（P2）

```ts
interface LibraryBooking {
  id: string;
  seat: string;
  area: string;
  time: string;
  status: string;
  actions: ('cancel' | 'checkIn' | 'leave' | 'return')[];
}
interface LibraryMyBooking {
  booking: LibraryBooking | null;
}
```

参考项目从 `/my/` 页面解析预约和可执行动作。图书馆系统只能在校园网直连下访问。

### 4.4 新增命令

| 命令               | 签名（前端 `bridge`）                                                                                | 规则                                                                                                               |
| ------------------ | ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `lms_preview_file` | `(uploadId) → FilePreview`                                                                           | 图片 ≤20 MB 返回 data URL；文本 ≤1 MB 返回文本（超出只取前 1 MB）；PDF ≤50 MB 交给 pdf.js 渲染；其他 `unsupported` |
| `lms_save_files`   | `(items: { uploadId; subdir?: string }[], dir) → { saved: number; failed: { uploadId; message }[] }` | 最多 50 个；单个上限 2 GB；同名追加“ (2)”；事件 `lms-download-progress { uploadId, written, total }`               |
| `campus_image`     | `(url) → { mime; dataUrl }`                                                                          | 只允许 `dean.xjtu.edu.cn`；≤10 MB；用于校历                                                                        |
| `campus_save_file` | `(url, path) → number`                                                                               | 只允许通知栏目主机名；≤500 MB；路径来自保存对话框                                                                  |
| `export_calendar`  | `(path, events: CalendarEvent[]) → number`                                                           | 生成 RFC 5545，`TZID=Asia/Shanghai`，字段转义；≤2000 条；路径来自保存对话框                                        |
| `save_text_file`   | `(path, content) → number`                                                                           | CSV 导出用；≤10 MB；路径来自保存对话框                                                                             |
| `school_write_*`   | 见 4.5                                                                                               |                                                                                                                    |

```ts
type FilePreview =
  | { kind: 'image'; name: string; mime: string; size: number; dataUrl: string }
  | { kind: 'text'; name: string; mime: string; size: number; text: string; truncated: boolean }
  | { kind: 'pdf'; name: string; size: number; previewId: string }
  | { kind: 'unsupported'; name: string; size: number | null };

interface CalendarEvent {
  title: string;
  start: string; // YYYY-MM-DDTHH:mm，本地时间
  end: string;
  location?: string;
  note?: string;
}
```

### 4.5 写操作统一模型

评教保留现有三条命令，不迁移，避免改动已接通的写路径。其他写操作统一走：

```ts
type SchoolWrite =
  | { kind: 'lmsHomework'; activity: number; comment: string; files: string[]; draft: boolean }
  | { kind: 'lmsRetract'; submission: number }                       // 待探测
  | { kind: 'libraryBook'; area: string; seat: string }              // P2
  | { kind: 'libraryAction'; booking: string; action: 'cancel' | 'checkIn' | 'leave' | 'return' } // P2
  | { kind: 'venueBook'; venue: number; date: string; stocks: number[] } // P2
  | { kind: 'venueCancel'; order: string };                          // P2

interface WritePreview {
  id: string;
  kind: SchoolWrite['kind'];
  accountId: string;
  expiresInSeconds: number;
  title: string;
  summary: { label: string; value: string }[];
  files?: { name: string; size: number }[];
  warnings: string[];
  irreversible: boolean;
  captcha?: { image: string; kind: 'slider' | 'text' }; // 场馆预订，待探测
}

interface WriteOutcome {
  status: 'done' | 'rejected' | 'unknown';
  message: string;
  detail?: Record<string, unknown>;
}

bridge.schoolWritePrepare(request: SchoolWrite): Promise<WritePreview>;
bridge.schoolWriteCommit(id: string, captcha?: unknown): Promise<WriteOutcome>;
bridge.schoolWriteCancel(id: string): Promise<void>;
```

规则：

1. `prepare` 不改动学校任何数据：只校验参数和权限，读取对象当前状态，解析本地文件。
2. 预览 id 只能用一次，有效 5 分钟，绑定账号和代次；账号切换、重新认证后作废。
3. 作业文件只来自系统对话框或拖放，页面不接受手输路径。`prepare` 记录每个文件的大小和修改时间，`commit` 前重新核对，有变化则拒绝并要求重新预览。
4. `commit` 只提交一次。网络中断、无法识别的响应、5xx 都返回 `unknown`，不自动重试；界面引导用户刷新提交记录核对，不提供“重试”按钮。
5. `cancel` 只丢弃尚未执行的预览，不宣称撤回已提交的内容。
6. `irreversible` 为真的操作（正式提交作业、取消场馆订单），确认按钮旁说明“提交后能否撤回以学校设置为准”。
7. 验证码由用户本人在预览面板中完成，后端只转发用户的操作，不做自动识别。
8. Agent 的 `campus_lms_submit` 改为调用同一套 `prepare` / `commit`，保证页面和 Agent 只有一条写路径。
9. 结果写入本地操作记录（X-07）。

### 4.6 本地状态

全局配置（`AppConfig`）新增：

| 字段                  | 类型                                     | 用途               |
| --------------------- | ---------------------------------------- | ------------------ |
| `campusSection`       | `string`                                 | 记住上次打开的分区 |
| `campusRailCollapsed` | `boolean`                                | 分区导航收起状态   |
| `lmsDownloadDir`      | `string \| null`                         | 上次下载目录       |
| `roomFavorites`       | `{ campus: string; building: string }[]` | 常用楼宇           |
| `libraryFavorites`    | `string[]`                               | 常用图书馆区域     |
| `homeNoticeChannels`  | `string[]`                               | 主页通知栏目       |
| `cardLowBalance`      | `number \| null`                         | 低余额阈值（分）   |

`campusPinned` 保留，改为主页“常用功能”的数据来源。

按账号保存（和校园缓存一起隔离，不放全局配置）：通知已读 / 收藏、已下载文件标记、“新内容”判断用的上次快照、操作记录。

### 4.7 前端派生计算（`src/domain.ts`，需单元测试）

- `gradeStats(rows)`：
  - 已修学分 = `passed !== false` 且 `credits > 0` 的学分之和
  - 不及格学分 / 门数 = `passed === false` 的学分 / 门数
  - 加权绩点 = Σ(`gpa` × `credits`) / Σ`credits`，只计 `gpa != null` 且 `credits > 0` 的行
  - 加权均分 = Σ(分数 × `credits`) / Σ`credits`，只计分数能解析为 0–100 数字的行；等级制成绩（如“A＋”）计入“未计入均分门数”
  - 用 `CXCKDM`（初修 / 重修）区分记录；同一课程有重修记录时默认只计最新一次，界面注明口径，可切换“全部计入”。学校官方口径未知，指标旁标“按本页数据计算”
- `todoBuckets(items, now)`：按 Asia/Shanghai 的日期分组；`dueAt == null` 归入“无截止”。
- `dueTone(dueAt, now)`：已过 → danger；3 小时内 → danger；24 小时内 → warning；其他 → neutral。
- `parseExamTime(kssjms)`：解析 `YYYY-MM-DD HH:MM-HH:MM(星期X)` 为开始、结束时间（Asia/Shanghai）；解析失败返回 `null`，不显示倒计时。
- `sectionTime(periods, section)`：用课表接口返回的 `periods` 把节次换成钟点；`periods` 缺失时退回只显示节次，不在前端写死作息表。
- `lmsTime(value)`：思源学堂时间都是 UTC ISO（`…Z`），显示和计算前一律转成 Asia/Shanghai。
- `attendanceStats(rows)`：
  - 到课率 = (正常 + 迟到) / (正常 + 迟到 + 缺勤)
  - 准时率 = 正常 / (正常 + 迟到 + 缺勤)
  - 请假、无需考勤、待定、未知不计入分母

### 4.8 账号与安全

- 新查询、新缓存、本地状态都按账号隔离；切换账号时关闭阅读器、预览窗口、播放器和写操作预览。
- 文件预览不执行任何内容：HTML 附件按文本显示，SVG 按图片显示（不内联到 DOM）。
- 下载和导出的路径只来自系统对话框。
- 日志里不出现 Cookie、令牌、文件内容和学籍敏感字段。

### 4.9 探测结果（2026-10-30，真实账号只读探测）

探测只发查询请求，只记录字段名、值类型、数量、格式和域名，不记录 Cookie、令牌和个人数据。临时探测代码不进仓库。

| 项目                   | 要确认什么                                                                                                                                                                                                                                                                                                                                                                              | 影响                    |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------- |
| LMS 活动列表           | **已确认**：列表带 `is_closed`、`is_in_progress`、`deadline`、`submit_times`、`late_submission_count`、`updated_at`，**不带** `user_submit_count`（只在详情里）。有官方待办接口 `GET /api/todos`：一次请求返回所有课程中未提交、未截止的作业（本账号 11 条），字段 `id`、`course_id`、`course_name`、`course_code`、`title`、`type`、`end_time`、`is_locked`。时间均为 UTC ISO（`…Z`）  | `LmsTodos` 请求量       |
| LMS 回放               | **已确认**：`player-url` 的 token → RMS `embed-token` → `captures` 返回 `lesson_videos`（每节课 2 路，字段 `label`、`size`、`mute`、`is_best_audio`、`play_type`、`play_url`、`download_url`）。`download_url` 是 `review-class.xjtu.edu.cn` 的 S3 预签名地址，不带 Cookie 也能取，支持 Range（206），可直接给 `<video>` 播放，但会过期（`X-Amz-Expires`）；`play_url` 不带会话返回 403 | LMS-12、CSP             |
| PDF 预览               | 已定用 pdf.js（见第 6 节）；确认锁定版本、worker 加载方式和大文件性能                                                                                                                                                                                                                                                                                                                   | LMS-07                  |
| 考勤记录               | **已确认**：有 `courseName`、`courseCode`、`courseId`、`startTime`、`endTime`、`appealEnabled`、`appealStatus`、`resultReason`、`teachers`；近 120 天 48 条，出现过的状态只有 `NORMAL`、`ABSENT`。响应里还有班主任、辅导员等个人字段，后端不要透传                                                                                                                                      | ATT-03                  |
| 通知正文               | **已确认（2026-10-30，公开页面实测）**：7 个栏目同一套站群系统，正文都在 `form[name=_newscontent_fromname]` 内的 `[id^=vsb_content]`（学生处为 `#vsb_content_2`，研究生院没有 `.v_news_content`）；附件是同一种下载链接；页面 17–118 KB，图片 6–21 张（含站点装饰图，需只取正文内的）                                                                                                   | NTC-02                  |
| 作息时间               | **已确认**：课表接口 `timetable/weekly` 本身返回 `periods`（`periodNo`、`startTime`、`endTime`，11 节），10 月实测为冬季作息（第 5 节 14:00）。直接用学校返回值，不在前端写死作息表；夏季是否自动切换等 5 月再观察                                                                                                                                                                      | HOME-01、TT-01、ROOM-01 |
| 考试时间               | **已确认**：`KSSJMS` 形如 `YYYY-MM-DD HH:MM-HH:MM(星期X)`，`KSRQ` 形如 `YYYY-MM-DD HH:MM:SS`。当前学期考试尚未公布（0 条），上学期 8 条                                                                                                                                                                                                                                                 | EXM-01                  |
| 图书馆                 | **未测**：账号当前是 WebVPN 模式，没有图书馆会话；需要在校园网下打开一次图书馆页面再测。预约后签到时限只能在真实预约后确认                                                                                                                                                                                                                                                              | LIB-04、LIB-05          |
| 场馆                   | **未测**：同上，需要校园网。支付方式参考项目显示为单独的支付页                                                                                                                                                                                                                                                                                                                          | VEN-04                  |
| 成绩                   | **已确认**：同一个成绩响应里就有构成明细，不需要新接口：`PSCJ` 平时（26/45 门有值）、`QMCJ` 期末（39/45）、`SYCJ` 实验（12/45）、`QTCJ1–3` 其他、`PSCJXS` / `QMCJXS` 占比；`CXCKDM` 标识初修 / 重修（45/45 有值）；本账号成绩全部为数字                                                                                                                                                 | GRD-01、GRD-08          |
| LMS 撤回 / 讨论 / 问卷 | **未测**：只读探测拿不到撤回和提交接口；讨论详情只有 `topic_count`，问卷详情只有 `is_anonymous`、`publish_count`，正文需要另外的接口                                                                                                                                                                                                                                                    | LMS-13、LMS-14          |

## 5. 验收口径

- 每个功能分别记录：代码实现 / 自动测试通过 / 真实账号验证 / 用户验收。
- `domain.ts` 的派生计算都要有单元测试；Rust 解析使用人工编造、已脱敏的测试数据，真实响应不进仓库。
- 写操作测试覆盖：预览不改数据、预览过期、账号切换后作废、文件变化后拒绝、结果不明不重试。
- 图书馆和场馆只能在校园网直连下验证；未验证前界面和文档都要如实说明。

## 6. 已确认的决定（2026-10-30）

| #   | 问题           | 决定                                                                                                                                                                       |
| --- | -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | 信息架构       | **方案 A**：畅行校园一个入口，内部分区导航（LAYOUT §2.1）。方案 B 不再考虑。                                                                                               |
| 2   | P0 范围        | 同意第 3 节标注的 P0。                                                                                                                                                     |
| 3   | PDF 预览与回放 | 同意放宽 CSP、新增依赖。PDF 采用 pdf.js（精确版本锁定），CSP 只增加 `worker-src 'self'`；回放增加 `media-src`，只放行白名单主机。`frame-src`、`object-src` 保持 `'none'`。 |
| 4   | 场馆预订验证码 | 接受：用户本人在应用内完成验证码，不做自动识别。                                                                                                                           |
| 5   | 一网通办       | 移除，不保留兜底入口。                                                                                                                                                     |
| 6   | 系统通知       | 引入 Tauri 通知插件（精确版本锁定），X-08 升为 P1。                                                                                                                        |

这些决定只确定范围和技术方向，**不代表视觉获准**。下一步按 LAYOUT §11 出参考图，用户确认图稿并同意开始实现后再改前端。
