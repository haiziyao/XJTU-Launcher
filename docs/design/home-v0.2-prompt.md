# 主页设计稿第二版

状态：用户于 2026-10-19 确认此版主页风格与侧栏展开 / 收起方案。仅主页及共用侧栏获准按图实现，其他页面（包括账户设置和登录表单）仍需参考图与用户确认。本记录不表示代码已完成。

后续修正：用户否决当前实现效果，认为仍像网页后台。当前前端实现已暂停，需重新出图确认；侧栏开关改为侧栏右侧分隔线中部的 `》/《`，不使用顶部工具栏开关。此前认可不能当作新修订稿的实现授权。

使用内置 image_gen 工具，以上一版设计图为编辑目标。姓名和学号均为设计占位。

## 实际生成提示词

+Use case: ui-mockup edit.
Edit the attached XJTU Launcher desktop homepage design into a revision sheet showing the SAME client in two sidebar states. Preserve its existing native desktop appearance, neutral light surfaces, graphite Chinese text, restrained system-blue accent, compact native controls, home content structure, simple line icons, app tiles, thin separators, and typography. The user says the overall desktop-client style is now approximately right; do not redesign the visual identity.

Composition: one high-resolution landscape design sheet, about 3072x1408, with TWO complete desktop application windows side by side, both large and readable. Equal outer window dimensions. The first has the expanded sidebar; the second has the collapsed sidebar. Outside the application windows place only small simple Chinese labels "侧栏展开" and "侧栏收起". Keep each screenshot convincing as a functional native client, not a website or presentation. No device mockup, no heavy drop shadows.

Requested change 1 — user area:
Replace the entire lower-left account-status row and its status dot with a compact, clean account area. In the expanded sidebar show a neutral round user silhouette avatar, then two text lines: "张同学" and "2026000000". These are explicit design placeholders for the logged-in student's actual name and student number. NO green, gray, red or any other online-status light, NO floating availability dot or status badge. No "校园账户未连接" label. Do not show a separate launcher platform account or a switch-account button.
Below the two windows, include one small alternate-state component sample with a neutral user avatar and exact label "未登录" to demonstrate what this same account area shows without login. No status dot there either. A small outside caption may say "姓名、学号为设计占位".
In both main screenshots use consistent signed-in placeholder state. The upper-right of the main content should simply say "西安交通大学"; remove the disconnected label, status dot, and connect button there. The today-course panel should say "尚未同步课表" with a small "刷新课表" button rather than asking a logged-in student to connect again. No fake courses.

Requested change 2 — collapsible navigation:
Make a clearly visible native sidebar-toggle icon button in the top toolbar immediately to the left of the current-view title "主页", aligned consistently in both screenshots. Use a panel-with-left-column icon, not a tiny ambiguous hamburger hidden in a menu. It stays visible in BOTH states.
Expanded screenshot: sidebar around 210px equivalent width, icon and label for each entry. Exact entry order is "主页", "Agent", small section label "插件", "课程表", "校园导航"; bottom anchored "插件市场", "账户设置", then the clean student account area.
Collapsed screenshot: sidebar is a narrow 56px icon rail, ALL navigation text and group headings are hidden. Keep the exact SAME icons in the exact SAME vertical order and the selected Home icon treatment. At bottom keep plugin-market icon, settings icon, and the avatar icon only. The main content expands horizontally to use the newly available width; do NOT leave an empty column where the expanded sidebar was. Keep the sidebar-toggle button visible in the toolbar so it can expand again. Do not show the app title overflowing the icon rail; the window title can sit in the top titlebar instead.

Other content stays the same: compact date/time row; "今日课程" panel with "打开课表"; 2x2 "校园快捷入口" tiles "一网通办", "教务系统", "思源学堂", "图书馆"; "我的插件" native list rows "课程表" and "校园导航", versions, enabled toggles, chevrons; "自定义主页" toolbar button; bottom status bar with "本地存储", "2 个插件", "V0.1". Replace the original invented plugin descriptions with accurate short text: course plugin "查看学期课表与今日课程"; navigation plugin "打开常用校园网站". No reminders or route-planning claims.
Constraints: change only the requested account treatment, sidebar toggle/state, and consistency corrections. No marketing slogans, hero banner, promotional English, decorative illustration, large website navigation, new features, or online status lights. Correct readable Chinese.
