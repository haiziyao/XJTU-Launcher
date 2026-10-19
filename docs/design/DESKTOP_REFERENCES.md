# 桌面界面参考研究

日期：2026-10-19。当前为参考研究，未修改界面代码；以下建议尚待设计图与用户确认。

## XJTUToolBox：校园业务与应用导航

- 上游：https://github.com/yan-xiaoo/XJTUToolBox
- 本地：`/Users/bilibili/Developer/HZY/References/XJTUToolBox`
- 研究提交：`e6a2d3b603ff7a803cc1d658a1fb5d52ca6b511b`
- 已实际查看 `screenshots/sample_1.png` 至 `sample_4.png`。这是仓库提供的展示图，不能视为当前版本所有页面的实机截图。
- 已读 `app/main_window.py`：主窗口继承 `MSFluentWindow`，功能通过 `addSubInterface` 注册，账户及设置使用 `NavigationItemPosition.BOTTOM` 固定到底部。
- 可借鉴：稳定的应用侧栏；校园业务在主内容区域切换；表格上方集中放置相关操作；底部账户入口。
- 不建议照搬：主页巨大彩色入口、大标题与大留白。它的现成主页结构不能替代本项目的可 DIY 主页。

## Clash Verge Rev：Tauri 客户端外壳

- 上游：https://github.com/clash-verge-rev/clash-verge-rev
- 本地：`/Users/bilibili/Developer/HZY/References/clash-verge-rev`
- 本次浅克隆提交：`79657119452b8202a4f1daf64acb12e7cf093080`
- 已实际查看官方 `docs/preview_light.png`，并核对布局源码；截图与当前开发分支可能存在差异。
- `src/components/layout/layout-sidebar.tsx`：主导航、底部流量区域分开；右键菜单提供展开/收起，`collapse_navbar` 保存状态；菜单可以解锁排序。
- `src/pages/_layout.tsx`：根据设置应用收起状态类；按平台处理布局。
- `src/assets/styles/layout.scss`：窗口整体固定占满，主区域 flex 分栏、外层禁止滚动，导航独立滚动，细分隔线明确边界。
- `src/components/layout/window-controller.tsx`：通过 Tauri 窗口 API 处理窗口行为，并考虑操作系统差异。是否使用哪类窗口控件须结合具体平台配置核对，不能仅根据此组件宣称全平台都采用系统按钮。
- 可借鉴：窗口、侧栏和内容区域构成一个连续界面；局部工具栏贴近内容；侧栏状态持久化；功能多时导航独立滚动。
- 不直接照搬：品牌区尺寸、代理业务列表及其图标；它的收起入口在右键菜单，不是用户要求的分隔线中部按钮。

## 本项目下一版设计建议（未批准）

1. 以 Clash Verge Rev 的窗口分区为结构参考，结合已认可的浅色风格，缩减品牌区与装饰性空白；保留 DIY 主页内容。
2. 侧栏按已确认顺序排列：主页、Agent、插件；底部插件市场、账户设置。底部用户信息展示头像、姓名和学号，未登录仅显示“未登录”，不加状态灯。
3. 按用户要求把收起开关放到分隔线垂直中部，以统一图标组件显示方向。收起后保留图标栏及展开入口。此位置来自用户要求，不声称是上述项目现有交互。
4. 下一张参考图同时给出展开与收起状态，供用户审阅；确认之前不修改前端。账户设置和 Agent 等其他页面另行设计。

## 研究边界

也查看了 Cherry Studio 官方 README，但截图链接未能可靠打开，因此本次不依据它作视觉推荐。未运行这些参考应用，也未复制其界面源码。当前记录的是已看见的截图与已核对的实现。
