---
name: xjtul-push
description: 提交、推送、合并分支与发布 XJTU Launcher 时使用。包含多人协作分支流程、提交身份、推送前检查、PR 与 tested 标签、版本号、Release 记录和 OSS 更新。
---

# 推送、合并与发版

## 分支流程（2026-11-11 起）

- 分支命名**不做要求**，可以按主题或 issue 自建。
- 每个功能或 bug 修完后，**必须并入当前版本分支**，不要长期留在个人分支。
- **不要凭记忆假设分支名**；开始前先跑 `git branch -a`，文档里的分支名可能已经过时。
- 版本分支开发期可自由提交；到发版时间点冻结，不再接受新功能。
- 冻结后把版本分支同步到 `dev`，由测试人员做集成测试。`dev` 只做集成与测试，不直接开发。
- 测试通过后由 `dev` 创建 PR 合入 `main`，测试人员给 PR 加 `tested` 标签；`main` 更新后自动构建发布。
- `main` 是发布分支，受保护：禁止直接推送、删除、强推，只接受 `dev` 来源的 PR。

## 提交身份

提交前确认使用的是**你自己的** Git 身份，不要使用项目维护者或他人的姓名、邮箱：

```sh
git config --show-origin --get user.name
git config --show-origin --get user.email
```

没有配置时，用自己的信息设置仓库级配置：

```sh
git config --local user.name "<你的名字>"
git config --local user.email "<你的邮箱>"
```

不要把任何人的真实姓名、邮箱写进共享文档、skill 或提交模板。

不要提交密码、Cookie、API Key、令牌、私钥、`~/.xjtu-launcher/auth.json` 或真实账号数据。

## 推送前检查

至少跑与改动相关的检查；较大一批修改统一执行：

```sh
npm test
npm run build
npm run format:check
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

只改文档时也检查链接和图片路径。确保没有把未提交的调试代码、临时截图、真实账号信息带进提交。

## 提交与推送

```sh
git status
git diff --check
git add -A
git commit -m "feat: 说明改了什么"
git push -u origin <分支名>
```

- 功能代码与开发资料尽量分开提交，按功能提交。
- 需要隔离改动时用 worktree，不要随意覆盖当前工作。
- 推送个人分支和合入版本分支可以随任务执行；合入 `dev` / `main`、发版和撤回 Release 仍按用户当次明确要求执行。
- `main` 永远不要 `git push --force`。

## 合入版本分支

```sh
git switch <当前版本分支>
git merge --no-ff <你的分支>
git push origin <当前版本分支>
```

冲突时保留双方意图，重新跑检查；不要为了省事直接丢弃别人的改动。

## 发版

1. **冻结版本分支**：确认没有未完成的功能；从最新 `main` 拉出版本分支（如果还没拉）。
2. **同步版本号**：同时修改 `package.json`、`package-lock.json`（两处）、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`、`src-tauri/tauri.conf.json`。`tools/prepare-release.mjs` 会校验一致性。
3. **写版本记录**：按 [模板](../../docs/releases/TEMPLATE.md) 新建 `docs/releases/v<版本>.md`，登记到 [索引](../../docs/releases/README.md)。自动发版会把这份文件当作 Release 正文；文件缺失或含 `YYYY-MM-DD` 占位符会中止发布。
4. **同步入口**：根 README 的三条下载链接、`website/downloads.json`、`dev-docs/CONVENTIONS.md` 的当前版本一起更新。
5. **合入 `dev` 集成测试**：把版本分支同步到 `dev` 并推送，交给测试人员验证真实登录、关键流程与安装包。
6. **合入 `main`**：测试通过后由 `dev` 创建 PR 合入 `main`，给 PR 加 `tested` 标签。`main-guard` 会拒绝非 `dev` 来源的 PR。
7. **自动发版**：`main` 更新后 `release.yml` 自动执行版本检查、测试、三平台串行构建、macOS 启动冒烟、资产校验、发布并标记 Latest；`oss` job 上传到私有 Bucket，`latest.json` 指向下载网关。
8. **发布后核对**：下载资产核对 `sha256`、macOS 包签名与启动、`latest.json` 的六个平台地址和 `notes`；官网 `downloads.json` 与 README 链接必须指向同一版本。
9. **收尾**：把版本记录的“验证”改成实际结果，追加 `dev-docs/DEVELOPMENT.md` 记录；把 `main` 合回 `dev`。

## 撤回

- 删除 Release 与标签，删除镜像 / OSS 对应目录，版本记录状态改为“已撤回”并写明原因与替代版本。
- 更新 README、版本索引与 `latest.json`，不要让客户端继续指向坏版本。
- 已安装用户无法自动更新时，在 README 给出覆盖安装说明。

## 发版前最后确认

- 版本号五处一致。
- `docs/releases/v<版本>.md` 存在、无占位符、已登记索引。
- README 下载链接、`website/downloads.json`、`latest.json` 预期一致。
- 三平台构建成功；macOS 启动冒烟通过；Windows 至少说明是否真机验证。
- 没有把“代码实现”写成“用户验收”。
