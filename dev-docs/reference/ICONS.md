# 自定义入口图标

侧边栏的入口图标可以由用户自己替换，不需要界面操作：把图片放进隐藏目录，再在 `icons.json` 里写一行映射即可。

## 目录

```text
~/.xjtu-launcher/resources/icons/
├── icons.json          入口 id → 图片文件
├── home.png
└── campus/todo.png     允许子文件夹
```

首次启动会创建这个文件夹和一份模板 `icons.json`，里面列出当前所有可用的入口 id，值留空（留空 = 使用内置图标）。模板只写一次，之后应用**不再改写这个文件**——包括用户在设置页改任何选项，都不会动它。文件属于用户。

## 文件格式

```json
{
  "version": 1,
  "map": {
    "nav.home": "home.png",
    "plugin.cn.xjtu.campus": "campus.png",
    "campus.todo": "campus/todo.png"
  }
}
```

- `version` 目前只有 `1`；不是 1 时整份映射被忽略。
- `map` 的键是**稳定的入口 id**，不是界面文案，改名不影响映射。
- 值是相对 `resources/icons/` 的路径；不允许 `..`、绝对路径、盘符。
- 值为空字符串表示保留内置图标，方便模板自解释。

## 可用 id

| 前缀               | 来源                                                     | 例子                                                                        |
| ------------------ | -------------------------------------------------------- | --------------------------------------------------------------------------- |
| `nav.<id>`         | 主侧边栏：`home` `agent` `community` `market` `settings` | `nav.home`                                                                  |
| `plugin.<插件 id>` | 侧边栏里的插件入口，含畅行校园、课程表等内置插件         | `plugin.cn.xjtu.campus`、`plugin.cn.xjtu.timetable`                         |
| `campus.<id>`      | 畅行校园左侧分区                                         | `campus.todo` `campus.lms` `campus.grades` `campus.card` `campus.fitness` … |

完整列表见首次生成的模板；新增入口时同步更新 `src-tauri/src/features/icons.rs` 的 `KNOWN_IDS`，否则模板里不会出现，但手写映射仍然有效。

## 图片要求

- 格式：PNG、JPEG、WebP（按文件内容识别，不看扩展名）。不支持 SVG：现有 `image` 依赖只编译了这三种解码器，且 SVG 可内嵌脚本。
- 单文件不超过 8 MB，边长不超过 4096 像素。
- 后端读取后居中裁成正方形并缩放到 96×96，统一转成 PNG 数据 URL 交给前端；原图多大都不会进入 webview。
- 显示尺寸跟随内置图标：主侧边栏 20px，畅行校园分区 17px，四角 24% 圆角。

## 容错

- 映射里的某一项找不到文件、格式不对、过大：只跳过这一项，入口回落内置图标，其余照常生效。
- `icons.json` 不是合法 JSON、或 `version` 不对：整份映射不用，全部入口回落内置图标；应用不会因为这个文件无法启动或无法保存设置。
- 前端只在启动和打开「应用设置」时重新读取，所以改完文件后重开一次设置页即可看到，不用重启应用。

## 实现位置

| 位置                              | 作用                                                         |
| --------------------------------- | ------------------------------------------------------------ |
| `src-tauri/src/features/icons.rs`          | 模板生成、映射校验、解码缩放、返回 `{ icons, problems }`     |
| `src-tauri/src/lib.rs`            | `entry_icons` 命令，只允许主窗口调用，始终返回结果而不是报错 |
| `src/shared/icons.ts`                    | 一次性拉取并缓存 id → data URL，失败静默                     |
| `src/components/ui/EntryIcon.vue` | 有映射画 `<img>`，否则画传入的内置图标；尺寸与内置图标一致   |

## 验收

- 自动测试：`cargo test --lib icons::` 覆盖模板、路径逃逸、损坏项隔离、版本错误、缩放结果。
- 需要真机验证：把 `icons.json` 和图片放进去，打开应用设置再关闭，侧边栏对应入口应换成图片；删掉映射后重开设置页应恢复内置图标。

更新日期：2026-11-08。
