# 官网源码

`xjtu-launcher.cn` 的静态页面，没有构建步骤，也没有外部依赖。

```bash
# 只预览页面（点赞与下载量会显示不出来）
python3 -m http.server 8899

# 完整预览：先跑接口服务（仓库根目录）
STATS_DIR=/tmp/xjtu-stats python3 tools/site-api/site-api.py
# 再跑一个把 /api/ 转发到 127.0.0.1:8788 的静态服务；没有代理时页面只是隐藏计数
```

- 下载信息集中在 `downloads.json`：版本、日期、下载基址、文件名、大小与 SHA256。
- `base_url` 现在是站内路径 `/download/v<版本>/`，由服务器 nginx 转给签名网关再 302 到私有 OSS。本地 `python3 -m http.server` 预览时点下载会 404，只有部署到服务器后才会走通。
- 首页三张方向卡片的点赞数、右上角下载量来自 `/api/stats`；点赞走 `/api/like`，实现在 `tools/site-api/`。
- Q&A 页面在 `doc/qa/`，导航「Q&A」与页脚都指向它；左侧目录（`qa.js`）默认高亮「常见问题」，点击可展开并定位到具体问题。
- 部署、nginx、HTTPS 与 OSS 流程见 [dev-docs/operations/WEBSITE.md](../dev-docs/operations/WEBSITE.md)。服务器没有 `rsync`，用 `dev-docs/operations/WEBSITE.md` 里的 tar over SSH 命令同步。
