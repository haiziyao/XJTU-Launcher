# 官网点赞与下载量接口

`site-api.py` 是纯标准库的 Python 3.6 服务，只监听 `127.0.0.1:8788`，由 nginx 反向代理，公开路径为 `/api/`。

- `GET /stats`：`{"downloads": {"total": 46}, "likes": {"powerful": 0, "community": 0, "design": 0}}`
- `POST /like`：请求体 `{"key": "powerful" | "community" | "design"}`，返回最新票数；同一地址对同一方向只能投一次。
- `GET /healthz`：存活检查。

## 数据与隐私

- 点赞数存在 `STATS_DIR/likes.json`（服务器上 `/srv/xjtu-launcher/stats`，属主 `www`）。写入走临时文件 + `os.replace`，进程内加锁。
- 访客地址不落盘：先与随机盐一起做 SHA256，只保留 32 位十六进制摘要，盐也存在 `likes.json` 里。
- 下载量来自 GitHub Releases API，按安装包后缀（`.dmg` `.exe` `.msi` `.AppImage` `.deb`）累加，不统计 `latest.json` 与签名文件；缓存 600 秒，并把最近一次真实值写到 `downloads.json`。GitHub 不可达时返回上一次的真实值；连缓存都没有则返回 `null`，页面隐藏下载量，不显示编造的数字。
- 需要更高频率时可设置 `GITHUB_TOKEN` 环境变量（未登录的 GitHub API 每小时 60 次，缓存 10 分钟后绰绰有余）。

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
