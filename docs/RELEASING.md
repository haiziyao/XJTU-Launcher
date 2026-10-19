# 版本发布

Windows 适配分支为 `codex/windows-support`，同时维护三平台打包配置。界面与校园业务沿用主线。

## 发布流程

1. 同步 `package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 与 `src-tauri/tauri.conf.json` 中的应用版本。
2. 新增 `docs/releases/v版本号.md`，记录变化、下载平台和已知限制。
3. 推送分支，确认 Desktop checks；可手动运行 Desktop release 验证安装包，手动运行仅生成 Actions artifacts。
4. 在验证过的提交上创建带说明的标签并推送，例如 `git tag -a v0.1.0 -m "XJTU Launcher v0.1.0"`、`git push origin v0.1.0`。
5. 标签触发四个构建任务：Windows x64、macOS ARM64 / x64、Linux x64。全部成功后校验安装包、创建草稿、上传附件，最后公开 Release。

首次发布暂不配置签名和自动更新，安装包通过 GitHub Releases 手动下载。`tools/prepare-release.mjs` 仍支持配置完整更新地址、公钥和签名私钥后的签名构建；启用自动更新时需另行配置 workflow 的密钥传递与更新清单上传。

## 失败处理

构建失败时不会公开 Release。修复后重新验证；已公开版本不移动标签、不替换安装包，递增补丁版本发布。若附件上传阶段失败，会保留草稿供检查，应恢复该草稿的上传和发布，不重复创建同名版本。

## 本地构建

```sh
npm ci
npm test
npm run app:build
```

Windows 使用 Visual Studio C++ Build Tools 和 WebView2，macOS 使用 Xcode Command Line Tools。Linux（Ubuntu 22.04）安装：

```sh
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libfuse2 xdg-utils
```

安装包在 `src-tauri/target/release/bundle/` 下。每个平台在对应系统上构建；Windows 安装配置位于 `src-tauri/tauri.windows.conf.json`，由 Tauri 自动合并。
