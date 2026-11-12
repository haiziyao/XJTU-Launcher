---
name: xjtul-plugin
description: 新增或修改 XJTU Launcher 的内置插件时使用。包含插件清单格式、内置 id 注册、Page 类型、侧栏入口、插件设置存储、安全边界与测试清单。
---

# 内置插件开发

## 先记住边界

- 插件中心只展示**随应用发布的内置插件**，没有远端市场、下载、安装、卸载或本地导入（2026-11-08 用户决定）。
- 插件由项目自己实现，随 App 版本发布；**没有通用插件运行时**，也不加载第三方代码。
- 插件只有启用开关；侧栏入口跟随启用状态。
- 插件不得获得主窗口 IPC 权限，不得读取或上传校园凭据。
- 新增页面视觉仍要先有参考图并经用户同意；插件身份不绕过设计审批。

完整格式见 [dev-docs/features/PLUGINS.md](../../dev-docs/features/PLUGINS.md)。

## 目录与注册

1. 新建 `plugins/<id>/plugin.json`，`id` 使用反向域名，例如 `cn.xjtu.my-plugin`。
2. 在 `src-tauri/src/plugins/manifest.rs`：
   - 增加 `pub const MY_PLUGIN_ID: &str = "cn.xjtu.my-plugin";`
   - 把 id 加入 `BUILTIN_IDS`。
   - 在 `builtin()` / 清单加载处用 `include_str!("../../../plugins/<id>/plugin.json")` 编入。
3. 如果引入新的页面类型：
   - 在 `src-tauri/src/plugins/manifest.rs` 的 `Page` 枚举增加变体，并补序列化 / 反序列化。
   - 在 `src/App.vue` 的页面路由里增加对应分支和组件。
   - 如果是需要保留的类型，补上“外部清单不得声明该类型”的校验。
4. 如果插件有设置：
   - 设置文件固定为 `~/.xjtu-launcher/plugins/settings/<id>.json`。
   - 在 `src-tauri/src/plugins/settings.rs` 增加按插件 id 的值域校验、默认值回退和原子写入。
   - 损坏或超大的设置文件只影响该插件，不能让应用或 `config.json` 加载失败。
5. 更新 [dev-docs/features/PLUGINS.md](../../dev-docs/features/PLUGINS.md) 的内置插件一览和说明。

## 清单格式

```json
{
  "schemaVersion": 1,
  "id": "cn.xjtu.example",
  "name": "示例插件",
  "version": "0.1.0",
  "category": "invention",
  "author": "<实际作者>",
  "license": "MIT",
  "credits": "可选：第三方素材或改编代码的署名与许可。",
  "description": "一句话说明",
  "details": "展开后的详细说明，最多 600 字。",
  "enabledByDefault": false,
  "icon": "sparkles",
  "page": { "type": "example" },
  "widget": false
}
```

字段约束：

- `schemaVersion` 必须为 `1`。
- `category`：`campus`（校园广场）/ `invention`（拓展发明）/ `lab`（创新小港）。
- `name` 1–30 字；`description` ≤300 字；`details` ≤600 字；`author` 必填，1–40 字；`credits` ≤300 字。
- `version` 是插件自己的 semver（`MAJOR.MINOR.PATCH`，可带 `-prerelease`），不跟应用版本绑定；改插件时递增。
- `license` 缺省 `MIT`，写插件自身代码的许可；第三方素材或改编代码的许可写进 `credits`，不要用 `license` 笼统覆盖。
- 静态元数据只写 `plugins/<id>/plugin.json`，不要复制进 `config.json` 或插件设置文件。
- `page.type` 必须是已登记的页面类型；新增类型要同步 Rust 与 Vue。
- `enabledByDefault` 只影响新配置；用户切换后以 `disabledPlugins` 为准。
- `sidebar: false` 表示不占侧栏入口，但仍可在插件中心启停。
- `widget: true` 才会出现在主页「我的插件」区，目前只有课程表。
- 清单文件最大 64 KiB；校验失败会让应用启动报错，不要静默跳过。

## 前端接入

- 插件页面组件放 `src/components/`，命名与现有页面一致。
- 侧栏入口由 `App.vue` 根据 `sortedPlugins` 和 `disabledPlugins` 生成；不要手写固定入口。
- 主页小组件只在 `widget: true` 时出现，并遵守主页分区顺序配置。
- 复用 `src/components/ui/`、现有 token 和图标；不要另造视觉系统。
- 插件页的写操作（如果有）必须“先预览、用户明确要求、再 `confirm=true`”；结果不明不重试。

## 安全与数据

- 不把密码、Cookie、API Key、令牌写进插件设置、日志或截图。
- 不把校园凭据发送给任何插件或外部服务。
- 文件访问限制在当前账号的工作区或明确的用户文件夹内，拒绝 `..`、隐藏凭据目录和符号链接越界。
- 插件设置只接受封闭的值域；颜色只接受 `#rrggbb`，字体名只接受安全字符，数字有上下限。
- 桌面宠物这类独立窗口只注册自己的少量命令，不获得主窗口 IPC。

## 测试与验收

```sh
npm run build
npm test
npm run format:check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

至少覆盖：

- 清单能被解析，字段越界会被拒绝。
- 内置 id 数量与注册表一致。
- 设置往返、非法值回退、损坏文件不影响其他插件。
- 关闭插件后入口消失、重新启用后恢复。
- 新页面在真实 Tauri 窗口打开一次（不是只看 `dev:frontend`）。

最后说明验收状态：代码实现 / 自动测试 / 真机验证 / 用户验收，不要混写。
