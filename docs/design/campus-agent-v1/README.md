# 校园插件与 Agent 设计稿 v1

状态：**未通过，等待调整意见**。2026-10-19 用户回复：“需要调整，我会说明”。

这些图片是使用 AppKit 绘制的静态参考稿，沿用已有客户端配色与侧栏比例。不是产品截图；内容均是示例，没有接入真实学校数据或模型。

- [插件市场](01-plugin-market.png)
- [一键评教](02-evaluation.png)
- [Agent 对话](03-agent.png)
- [MCP 设置分区](04-agent-settings.png)

生成方式：`swift docs/design/campus-agent-v1/render.swift docs/design/campus-agent-v1`。

图片未经确认，不作为实现依据。Skills、记忆、工具配置的具体子页面仍需补充设计。其他校园业务页也需要在确认时覆盖，不能因为市场列表中出现名称就默认获批。
