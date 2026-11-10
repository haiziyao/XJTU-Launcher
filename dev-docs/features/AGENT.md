# Agent 与 Agent 设置

更新于 2026-11-11。本文只记录当前有效的 Agent 边界；2026-10-23 的设计过程、旧五标签结构和当时的实现状态已归档到 [archive/AGENT_2026-10-23.md](../archive/AGENT_2026-10-23.md)。

## 当前结构

- Agent 配置是内置插件 `cn.xjtu.agent-settings`，页面分「基础设置 / 能力设置 / MCP 能力」三个标签。
- 「基础设置」控制侧栏是否显示 Agent 与 MCP 模式开关；MCP 打开后就地展开端口、地址、命令和工具数量。
- 「能力设置」集中模型接入、MCP 接入、Skills、角色、预设任务与全局记忆，使用列表 + 弹出卡片，不在页面里堆解释小字。
- 「MCP 能力」按分类列出内置工具，每行是名称、用途和开关；读 / 写 / 改 / 删用圆角小方块标识，图例只在第一个分组标题右侧出现一次。
- 侧栏 Agent 入口由 `agentSidebarVisible` 控制，默认显示；关闭后仍可从 Agent 设置右上角「打开 Agent」进入对话。
- 个人页「开放接口」、Agent 基础设置和「MCP 能力」三处读写同一份 MCP 配置，都展示端口、地址、命令和工具数量。

## 工具与写操作

- 当前内置 17 个工具：13 个只读查询 + 4 个写工具。
- 只读查询按领域合并，用 `action` 参数选择具体能力。
- 4 个写工具：`campus_timetable_edit`、`campus_lms_submit`、`campus_evaluations_submit`、`workbench_edit`。
- 写工具默认只返回预览；用户明确要求后才带 `confirm=true`。结果不明时不自动重试，也不宣称撤回。
- 未勾选的工具不进入 `tools/list`，调用会被拒绝；选择保存在 `mcpServerDisabledTools`。

## 数据位置

- MCP 配置：`~/.xjtu-launcher/state/mcp.json`。
- 对话：`~/.xjtu-launcher/agent/conversations.json`；角色、Skills 与预设任务在 `presets.json`；全局记忆在 `memory.json`。
- 模型 API Key 与其他凭据保存在 `~/.xjtu-launcher/auth.json`，不写日志、截图或仓库。
- MCP 只监听本机回环地址；启动本地 MCP 子进程时界面必须显示实际命令，不把目录里的命令当作可信输入静默执行。

## 相关文档

- 当前实现与验证记录：[DEVELOPMENT.md](../DEVELOPMENT.md)
- 内置插件格式：[PLUGINS.md](PLUGINS.md)
- 安全与写操作规则：[CONVENTIONS.md](../CONVENTIONS.md)
- 历史设计：[archive/AGENT_2026-10-23.md](../archive/AGENT_2026-10-23.md)
