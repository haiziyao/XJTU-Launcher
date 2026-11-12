# 随应用资源

本目录存放随应用发布的静态资源。界面不硬编码这些内容，运行时由 Rust 读取并通过命令返回。

## 目录

```text
resources/
└── community/
    ├── sites.json        资源站点目录
    └── icons/            站点图标（SVG / PNG / JPEG / WebP）
```

`src-tauri/tauri.conf.json` 的 `bundle.resources` 把 `resources/community/` 映射到打包后的 `community/`；开发时 tauri-build 也会把它复制到可执行文件旁，因此 `resource_dir()` 在开发与安装后都能找到。读取入口是 `src-tauri/src/features/resources.rs` 的 `resource_sites` 命令，前端对应 `bridge.resourceSites()`。

## sites.json

顶层字段固定为 `version`（当前必须为 `1`）、`updatedAt`（`YYYY-MM-DD`）、`categories` 与 `sites`。字段名使用 camelCase。

```json
{
  "id": "jwxt",
  "title": "教务系统",
  "url": "https://jwxt.xjtu.edu.cn/",
  "icon": "icons/jwxt.svg",
  "description": "选课、课表、成绩与考试安排。",
  "categories": ["study"],
  "tags": ["选课", "成绩"],
  "featured": true
}
```

- `id`：全目录唯一，只允许 ASCII 字母、数字、`-`、`_`，最长 64。
- `title`：1–60 字；`description`：最多 200 字；`tags`：最多 8 个，每个 1–16 字。
- `categories`：至少一个，必须引用 `categories` 中存在且不重复的 id。
- `featured`：为 `true` 的站点出现在「推荐」标签；其他站点只按分类展示。
- `url`：只接受 `http` / `https`，必须有主机名，不得包含用户名或密码。
- `icon`：相对 `community/` 的路径，只允许普通路径段；缺图或文件损坏时界面显示通用图标，不让整个目录失败。
- `categories` 的 `title` 为 1–30 字，最多 32 个分类、500 个站点；分类 id `recommended` 保留给界面的「推荐」标签，不能使用。

界面上的「推荐」标签固定排第一，其余标签按 `categories` 顺序排列。

## 用户覆盖

若 `~/.xjtu-launcher/resources/community/sites.json` 存在，应用优先读取它；其 `icons/` 也优先，找不到时回退到随应用发布的图标。该目录与内置目录使用同一套校验，校验失败会直接报错，不会静默退回内置目录。

## 修改约定

- 只登记能在项目中核实过的真实站点，不写假域名或占位链接。新增 URL 前先确认它来自现有代码、官方公开入口或用户提供的地址。
- 图标使用自绘的通用图形，不直接使用学校或第三方品牌素材。
- 改完 `sites.json` 或图标后运行 `cargo test --manifest-path src-tauri/Cargo.toml --locked`；打包验收时确认 `.app/Contents/Resources/community/` 中存在 `sites.json` 与 `icons/`。
