# 插件目录服务

运行 `npm run market:serve`，默认仅监听 `http://127.0.0.1:17321`。在客户端的“应用设置 → 插件服务”中点击“确认更改”，确认提示后填写该地址。远端市场将出现真实可安装的“学习资源导航”示例。

服务端无需访问校园账号、密码或 auth.json。插件文件在 `packages/`；添加或更新 JSON 后下一次请求即可获取。生产可设置 `PLUGIN_HOST`、`PLUGIN_PORT`、`PLUGIN_PACKAGE_DIR`，并在前方部署 HTTPS 反向代理。客户端正式默认地址由构建环境的 `XJTU_PLUGIN_SERVER` 指定；缺省为空，不会自动连接示例域名或本地测试服务器。

协议：

- `GET /catalog.json`：`schemaVersion: 1`、`plugins` 数组。
- 目录项：`id`、`name`、`description`、语义版本 `version`、`minAppVersion`、`downloadUrl`、十六进制 `sha256`。
- `downloadUrl` 指向同源的原始 plugin.json。包格式沿用 `dev-docs/PLUGINS.md`，当前远端只接受声明式导航，不支持 JS、Rust 动态库或任意命令。包上限 64 KiB，目录上限 1 MiB / 256 项。
- 客户端重新获取目录，核对选择的版本，验证包摘要、清单、兼容版本和来源；新安装先暂存后原子移动，更新原子替换包含清单与来源的单个文件。
- 已安装来源不同或同 ID 本地插件不会被覆盖；同版本和降级会被拒绝。
- 服务端重定向不被跟随；生产使用 HTTPS，本机开发允许 localhost / 127.0.0.1 / ::1 的 HTTP。

真实 HTTP 联调（不读写用户账户或真实插件目录）：

```sh
npm run market:serve
# 另一个终端
cargo run --manifest-path src-tauri/Cargo.toml --example market_smoke
```

示例检查目录、下载、摘要失败、原子安装、来源持久化与移除。无需登录学校。
