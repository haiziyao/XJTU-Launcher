# 项目开发 skill

本目录放 XJTU Launcher 的项目 skill，和源码一起维护，不依赖个人全局配置。skill 是操作手册，不替代 [开发约定手册](../dev-docs/CONVENTIONS.md)；两者冲突时以约定手册为准。

| skill          | 用途                                                    |
| -------------- | ------------------------------------------------------- |
| `xjtul-style`  | 改前端视觉、样式、组件前读；包含 token、组件与壁纸规则  |
| `xjtul-plugin` | 新增或修改内置插件；清单、页面路由、设置与安全边界      |
| `xjtul-push`   | 提交、推送、合并分支与发版；版本号、Release 与 OSS 流程 |

## 使用

- 在 Claude Code 里直接说“按 xjtul-style 做”“用 xjtul-push 走发版流程”，即可按名称加载对应 skill。
- 其他 agent 或人工阅读时，打开对应目录的 `SKILL.md`。
- skill 里的路径以仓库根目录为基准；调整目录结构时同步修改链接。
- 开始开发前仍要先读根目录 [dev.md](../dev.md)、[todo_list.md](../todo_list.md) 与 [acceptance_list.md](../acceptance_list.md)。

## 维护

- 改样式系统时同步 `xjtul-style`；改插件清单、注册或页面路由时同步 `xjtul-plugin`；改分支、提交、发布流程时同步 `xjtul-push`。
- 不写入具体版本分支名。长期分支只写 `main` 和 `dev`；版本分支名用占位符或让读者用 `git branch -a` 查。
- 不写入密钥、真实账号数据或未获准的设计结论。
