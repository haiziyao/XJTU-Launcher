# 版本 A：左侧分区栏工作区（已实现，2026-11-02 存档）

> 2026-11-02 用户要求“当前的模式记得保存好副本，记录下来。再开始新版本”，并同意用 A / B 区分。
> 本文记录版本 A 的实际界面与已知问题，供版本 B 对照。代码快照为 tag `campus-layout-a`，
> 排版规定原件见同目录 [LAYOUT-A-rail-2026-10-30.md](LAYOUT-A-rail-2026-10-30.md)。

## 界面结构

畅行校园（`src/components/CampusView.vue`）是一个工作区：

- 左侧 `CampusRail`：168px 宽，收起 48px（macOS 展开侧栏时可见），按分组排 17 个文字项。
  - 顶部无分组：待办、通知
  - 学习：思源学堂、资料库、课程资料
  - 学业：成绩、考试、评教、空闲教室、全校课程、校历
  - 生活：校园卡、图书馆、场馆、考勤、体测、学籍档案
- 右侧：`SectionHeader`（标题、副标题、更新时间、筛选、刷新、更多）+ 分区内容。
- 主要内容形态：`SplitView` 列表 / 详情、`ListItem` 整宽横条、`SettingGroup` 属性列表。
- 上次打开的分区记在 `localStorage` 的 `campus.lastSection`。

## 实际观察到的问题（2026-11-02 真机，1120×760）

1. **分区栏排不下**：17 项 × 32px + 4 个分组标题 + 间距约 640px，窗口高 760px 时最末项“学籍档案”贴到窗口底边，再窄就要滚动。
2. **文字密度低**：待办一屏 5–6 条；课程资料 36 个文件一屏 5–6 条，一行一条整宽横条，中间大片空白。
3. **课程没有信息量**：课程资料只按课程分出一条小标题，课程本身不显示文件数、时间、状态；思源学堂用下拉选课程，未选课程时才出现整宽课程行。
4. **选中态重叠**：点击新分区后，旧项保留粉色焦点框，新项是粉色底，同时出现两个高亮。
5. **与版本 A 的排版规定有偏差**：
   - 学籍档案按规定应在个人页（LAYOUT §2.1、§9.13），却仍放在校园“生活”组。
   - 课程资料是新增的侧栏项，规定里没有。
   - 待办 / 未读通知的数字角标（`RailItem.badge`）未接线。
6. 空状态与占位文字占满整块内容区，页面显得空。

## 版本 A 的既有实现（保留，不改）

组件、后端查询与接线见 [LAYOUT-A-rail-2026-10-30.md](LAYOUT-A-rail-2026-10-30.md) 与
[dev-docs/WIP-v0.6-frontend-kit.md](../../../dev-docs/WIP-v0.6-frontend-kit.md)；
页面组件全部在 `src/components/campus/`。

## 恢复方式

```bash
git diff campus-layout-a -- src/components/CampusView.vue src/components/campus
git checkout campus-layout-a -- src/components/CampusView.vue src/components/campus
```

截图不进入仓库：真机截图包含姓名与课表，只保留在本机临时目录。
