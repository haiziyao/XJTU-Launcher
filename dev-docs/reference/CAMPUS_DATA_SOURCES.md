# 校园数据接口储备

2026-11-12 调研。来源是体育平台与移动交通大学公开的前端页面和脚本（未使用账号登录实测），用于以后逐步接入；接口和字段可能随学校升级变化。2026-11-12 已按域名分类实现为 Rust 适配器，尚未接入命令、Agent 工具或界面。

凭据仍只保存在 `~/.xjtu-launcher/auth.json`，本文不记录任何 Cookie、令牌或密码。新增域名已补进 `src-tauri/src/core/network.rs` 的可信域名列表。

## 实现位置

| 文件                                                 | 内容                                             |
| ---------------------------------------------------- | ------------------------------------------------ |
| `src-tauri/src/school/direct.rs`                     | 直连 CAS 会话的共享 HTTP 客户端与可信跳转        |
| `src-tauri/src/school/sports/mod.rs`                 | 体育平台会话、`index.php` / `v3` 两代返回格式     |
| `src-tauri/src/school/sports/fitness.rs`             | 体测成绩、详情、视力、公告、预约、免测           |
| `src-tauri/src/school/sports/run.rs`                 | 乐跑规则、学期 / 近 7 天 / 明细、免跑            |
| `src-tauri/src/school/sports/course.rs`              | 体育课成绩、作业、学生信息                       |
| `src-tauri/src/school/sports/stadium.rs`             | 场馆列表、黑名单、订单、核销权限                 |
| `src-tauri/src/school/sports/contest.rs`             | 比赛列表、我的比赛、详情、报名 / 取消            |
| `src-tauri/src/school/portal/todo.rs`                | 待办 / 应用 / 其他数量、逾期提醒设置（transaction） |
| `src-tauri/src/school/portal/message.rs`             | 日程、消息、未读数（message-service）            |
| `src-tauri/src/school/portal/me.rs`                  | 门户个人资料（authx personal）                   |
| `src-tauri/src/school/portal/service.rs`             | 服务目录与分类（ywtb）                           |
| `src-tauri/src/school/portal/reservation.rs`         | 我的场馆 / 会议室预约                            |
| `src-tauri/src/school/portal/bus.rs`                 | 校车预约                                         |

现状边界：

- 只有体测学年列表用真实账号验证过；体测成绩明细仅做过页面验收，其余接口都来自前端代码，**未实测**。
- 所有写接口在适配器里只负责拼装请求。接入界面或 Agent 时必须"先预览、用户明确要求、再执行"，结果不明不自动重试。
- 乐跑与门户接口的返回结构尚未确认，适配器原样返回 JSON，不猜字段。

## 体育平台

站点：`https://tyxylp.xjtu.edu.cn/bdlp_h5_fitness_test/`。体测会话用 CAS 直连建立，`connect` 会用一次学年查询确认会话真的可用。

两代接口：

- `public/index.php/index/...`：表单 POST，返回 `{ status, data, info }`；`status == 1` 成功，`status == -1` 登录失效，其余情况 `info` 是提示文本。
- `v3/index.php/...`：乐跑数据。返回格式尚未确认，适配器原样返回。

### 体测成绩与详情（只读）

| 接口                            | 参数                        | 可得数据                                                                                                |
| ------------------------------- | --------------------------- | ------------------------------------------------------------------------------------------------------- |
| `Report/getStudentScore`        | `year_num`                  | 总分、总等级、报告类型 / 状态、免测提示；每项 `*_score` 原始值、`*_score2` 换算分、`*_grade`、`*_class` |
| `Report/getItemDetails`         | `item_num` 1–7、`year_num`  | `info.value_1` / `info.value_2` 原始测量值、`info.photo_url`（部分项目有体成分图）、`list` 历史成绩      |
| `Report/getEyeDetails`          | `year_num`、`from`、`student_num` | 视力数据；学生本人查询时后两项留空                                                                |
| `fitness/fitnessYear`           | `from=1`                    | 学年列表                                                                                                |
| `News/getSchoolNoticeList`      | `page`、`notice_type` 1/2、可选 `type` | 体育公告                                                                                     |

`item_num` 对应：1 身高体重（`value_1` 厘米、`value_2` 千克）、2 肺活量（毫升）、3 立定跳远（厘米）、4 坐位体前屈（厘米）、5 引体 / 仰卧（次）、6 50 米（秒）、7 800 / 1000 米。

这组数据足以支持真实比例的人体可视化（身高、体重、各项目原始值、历史记录、体成分图），不需要编造参数。

### 体测预约与免测

| 接口                                      | 用途                                     | 参数                       |
| ----------------------------------------- | ---------------------------------------- | -------------------------- |
| `fitness/getPlan`                         | 预约计划、说明、可选场次、黑名单状态     | 无                         |
| `fitness/getPlanByDate`                   | 指定日期与场地的可预约时段、容量         | `date`、`plan_id`、`location_id`；最早日期用 `2099-01-01` |
| `Appoint/getSchoolFitClassDetail`         | 预约前详情                               | `class_id`、`class_time`、`test_time`、可选 `appoint_id` |
| `Appoint/checkAppoint`                    | 预约前检查（不创建预约）                 | `class_id`、`class_time`、`location_id` |
| `appoint/tips`                            | 预约须知                                 | `school_id`                |
| `Appoint/setOnAppointment`                | 提交预约（写）                           | `class_id`、`class_time`、`location_id` |
| `Appoint/setOffAppointment`               | 取消预约（写）                           | `class_id`、`class_time`、`location_id` |
| `Free/getYear`                            | 免测申请学年；顶层还有 `freeType`、`miance`、`hideBmi` | 无              |
| `Free/setFree`                            | 提交免测申请（写）                       | `free_type`、`describe`、`photo`、`item`、可选 `stype` |

`Free/setFree` 的 `photo` 是上传步骤返回的文件名，不是本地路径；上传接口尚未确认。

### 乐跑（v3，未实测）

| 接口                              | 用途               |
| --------------------------------- | ------------------ |
| `Rule/running`                    | 乐跑规则           |
| `RunningData/termData`            | 本学期数据         |
| `RunningData/show7dataNew`        | 最近 7 天数据      |
| `Run/getTermDetail`               | 学期明细           |
| `Running/applyFreeRun`            | 免跑申请（写）     |

H5 页面在 URL 里带 `uid`、`school_id`、`token`、`sign`。适配器先按 CAS Cookie 会话发送，接入前用一个学期核对是否需要这些查询参数。

### 体育课、场馆、比赛

| 接口                                      | 用途                     | 参数                                    |
| ----------------------------------------- | ------------------------ | --------------------------------------- |
| `course/getStudentGymScore`               | 体育课成绩               | 无                                      |
| `Coursemanage/homework_list`              | 体育课作业               | `act=get`、`course_id`                  |
| `User/getStudentInfo`                     | 体育平台学生信息         | `type=1`                                |
| `Stadium/getStadiumList`                  | 场馆列表与开放信息       | 无                                      |
| `Stadium/checkBlack`                      | 场馆黑名单检查           | 无                                      |
| `stadium/orderDetails`                    | 订单详情                 | `order_id` 或 `order_num`               |
| `stadium/getScanAuth`                     | 扫码核销权限             | 无；`data.permission == 1` 表示允许     |
| `stadium/checkOrder`                      | 场馆下单（写）           | `order_id`、`details_id`、`school_id`   |
| `stadium/getSubOrderDetails`              | 子订单核销（写，工作人员） | `details_id`、`school_id`             |
| `match/match_list`                        | 比赛列表                 | `page`                                  |
| `match/mines`                             | 我的比赛                 | `page`                                  |
| `match/detail`                            | 比赛详情                 | `match_id`                              |
| `match/join`                              | 报名 / 取消（写）        | `match_id`、`type`（1 报名，2 取消）    |

体育课成绩与作业和思源学堂有重叠，接入前先决定是否合并展示。

## 移动交通大学（superapp）

`https://superapp.xjtu.edu.cn/` 只是外壳，数据分散在它挂接的独立域名，各自需要 CAS 会话。以下主机已加入可信域名：

| 系统         | 主机                          | 已确认路径                                                     | 内容                |
| ------------ | ----------------------------- | -------------------------------------------------------------- | ------------------- |
| 待办 / 交易  | `transaction.xjtu.edu.cn`     | `/ttc/api/ttc/center-list/getToDoListCountNew`、`getAppsCount`、`getOtherCount`；`/ttc/ttc/person-setting/getPersonOverdueSetting` | 待办与应用数量、逾期提醒 |
| 消息中心     | `message-service.xjtu.edu.cn` | `/center/api/v1/messageCenter/getPersonlCalendarList`、`listMessageRecord`、`getUnReadedCount`、`getMessageRecord` | 日程、消息、未读数 |
| 个人资料     | `authx-service.xjtu.edu.cn`   | `/personal/api/v1/personal/me/user`                            | 门户身份卡          |
| 服务目录     | `ywtb.xjtu.edu.cn`            | `/portal-api/api/proxy/service/api/rest/mobile/v2/selectMyServicesByTypeId`、`/portal-api/v2/service/category/list` | 服务与应用列表 |
| 场馆 / 会议室 | `reservation.xjtu.edu.cn`     | `/api/myreservation/my`                                        | 我的预约            |
| 校车预约     | `xjbus.xjtu.edu.cn`           | `/api/school/bus/user/pageReservationUsers`                    | 我的校车预约        |
| 表单流程     | `formflow.xjtu.edu.cn`        | 只知道基址 `/ttc/`，未找到可复用的 JSON 接口                   | 暂不实现            |

注意事项：

- 门户前端会合并重复斜杠后再发请求，适配器使用合并后的地址。
- 消息、个人资料等接口在 App 里还会带 `X-Id-Token` 等请求头。适配器目前只用 Cookie 会话，接入时先确认是否需要单独保存这些令牌。
- `reservation`、`bus`、`message` 的查询参数与写接口尚未确认，适配器把查询交给调用方，写操作等实测后再加。

## 接入顺序建议

1. 体测详情与历史（现有会话，只读，价值最高）。
2. 乐跑数据（现有会话，只读）。
3. 体测预约 / 免测（接口齐全，写操作按确认协议）。
4. 体育课成绩与作业（先确认与思源学堂的去重）。
5. 门户待办与消息（新增会话与请求头）。
6. 场馆预约、校车、一卡通（独立系统，最后做）。

## 注意

- 除体测学年列表外，接口均未用真实账号实测；接入前先用一个学年 / 一个学期核对返回。
- 页面路由与接口可能随学校升级变动，失败时按「结果不明」处理，不自动重试写操作。
- 新增域名、会话与缓存都要遵守账号隔离规则。
