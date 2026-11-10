# 官网点赞与下载量接口

`site-api.py` 是纯标准库的 Python 3.6 服务，只监听 `127.0.0.1:8788`，由 nginx 反向代理，公开路径为 `/api/`。

- `GET /stats`：`{"downloads": {"total": 46}, "likes": {"powerful": 0, "community": 0, "design": 0}, "voted": {"powerful": false, "community": false, "design": false}}`；`voted` 按请求携带的 `xl_voter` Cookie 计算，告诉页面这个浏览器自己投过哪些方向，与总票数无关。
- 每次投票、定时快照写 `likes.log` / `stats.log`，并把两份计数文件复制到 `backups/`，见下文「日志与备份」。
- `POST /like`：请求体 `{"key": "powerful" | "community" | "design"}`，返回 `{"ok": true, "key": ..., "count": 最新票数, "voted": true/false}`。同一浏览器对同一方向只能投一票，再发一次就是取消（`"voted": false`，票数减一）；未带投票 Cookie 的第一次请求会签发 `xl_voter`。
- `GET /healthz`：存活检查。

## 数据与隐私

- 点赞数存在 `STATS_DIR/likes.json`（服务器上 `/srv/xjtu-launcher/stats`，属主 `www`）。写入走临时文件 + `os.replace`，进程内加锁。
- 投票身份用第一方 Cookie `xl_voter`（`HttpOnly; SameSite=Lax`，两年有效），不用 IP：校园网和办公室 NAT 下很多人在同一个出口地址，按 IP 会把不同的人算成一票。
- Cookie 值不落盘：与随机盐一起做 SHA256，只保留 32 位十六进制摘要，盐也存在 `likes.json` 里；原始 Cookie 和 IP 都不写日志。
- 限制与取舍：换浏览器、隐身窗口或清 Cookie 会被当成新的人；脚本可以不断请求新 Cookie 刷票。nginx 的每 IP 30 次/分钟限流能压住一部分，这个投票只用于排优先级，不做防作弊承诺。
- 下载量 = GitHub Releases API 的历史计数 + 下载网关记录的 OSS 下载开始次数。GitHub 部分按安装包后缀（`.dmg` `.exe` `.msi` `.AppImage` `.deb`）累加，不统计 `latest.json` 与签名文件；缓存 600 秒，并把最近一次真实值写到 `downloads.json`。网关部分读同目录的 `gateway-downloads.json`（由 `tools/oss-gateway/gateway.py` 写入，只计不带 `Range` 的安装包 GET），文件不存在按 0 处理。GitHub 不可达时沿用上一次真实值；两边都没有值才返回 `null`，页面隐藏下载量，不显示编造的数字。
- 需要更高频率时可设置 `GITHUB_TOKEN` 环境变量（未登录的 GitHub API 每小时 60 次，缓存 10 分钟后绰绰有余）。

## 日志与备份

- `STATS_DIR/likes.log`：每次投票一行 JSON，`event` 是 `vote`（记了新票）或 `vote_duplicate`（同一地址重复点），带 `key`、`count`、`voter`（加盐哈希前 8 位，不含原始 IP）。写入失败或保存失败也记事件。
- `STATS_DIR/stats.log`：服务启动、GitHub 下载量抓取成功 / 失败、周期快照，带当时的点赞数与快照结果。
- 两个日志都按大小轮转：超过 5 MB 改成 `.1`，最多保留 5 个旧文件，不会无限增长。
- `STATS_DIR/backups/`：`snapshot_loop` 默认每 900 秒把 `likes.json` 与 `gateway-downloads.json` 各复制一份，文件名 `likes-YYYYmmddTHHMMSSZ.json` / `downloads-同格式.json`，每种各保留最近 192 份（约两天）。恢复时先停服务，把对应快照复制回原文件名再启动。
- 环境变量：`STATS_SNAPSHOT_INTERVAL`（秒，默认 900）、`STATS_SNAPSHOT_KEEP`（每种保留份数，默认 192）。点赞本身每次投票都立即原子写入 `likes.json`，快照是额外的兜底，不依赖快照保存。

## 安装（Alibaba Cloud Linux 3 / 宝塔）

```bash
mkdir -p /srv/xjtu-launcher/site-api /srv/xjtu-launcher/stats
cp site-api.py /srv/xjtu-launcher/site-api/site-api.py
chown -R www:www /srv/xjtu-launcher/stats
cp xjtu-site-api.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now xjtu-site-api
curl -s http://127.0.0.1:8788/stats
```

## nginx

`nginx-api.conf` 里两段都要用：

1. `http{}` 块的 `limit_req_zone`，追加到 `/www/server/nginx/conf/xjtu-launcher-limits.conf`。
2. 各 `server{}` 块里的反代：正式域名用 `/api/`，备案前的 IP 预览（站点在 `/site/`）用 `/site/api/`。

```bash
nginx -t && nginx -s reload
curl -s -H 'Host: 8.152.100.169' http://127.0.0.1/site/api/stats
curl -s -X POST -H 'Host: xjtu-launcher.cn' -H 'Content-Type: application/json' \
  -d '{"key":"community"}' http://127.0.0.1/api/like
```

测试投票会写进真实计数，验证完清掉：

```bash
rm -f /srv/xjtu-launcher/stats/likes.json && systemctl restart xjtu-site-api
```
