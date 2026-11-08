# OSS 下载网关

安装包放在私有 OSS Bucket 里，服务器只负责签发 60 秒有效的下载地址，并按浏览器 Cookie 限制每人下载次数；nginx 的按 IP 限流只做宽松兜底。这样即使链接被爬走，也不能无限刷流量。

## 为什么不直接公共读

公共读 Bucket 的对象 URL 是固定的，任何人拿到就能反复下载。OSS 按外网流出流量计费，没有硬性上限：一个脚本循环下载 25 MB 的包，一晚上就能跑出很高的账单。Referer 防盗链可以被伪造，也不能保护桌面客户端和 `curl`。

## 请求流程

```text
浏览器 / 应用
  → https://xjtu-launcher.cn/download/v0.5.2/XJTU.Launcher_0.5.2_aarch64.dmg
  → nginx：每个 IP 120 次/分钟、最多 50 个并发（只挡明显脚本）
  → 127.0.0.1:8787 校验路径与浏览器 Cookie 配额（默认 20 次/10 分钟），签发 60 秒有效的 OSS 地址
  → 302 到 https://xjtu-launcher.oss-cn-beijing.aliyuncs.com/...
  → OSS 直接把安装包发给用户，不经过服务器
```

限流只在「签发」这一步生效，文件传输仍然走 OSS 带宽。拿到签名链接的人在 60 秒内可以重复下载，这是这个方案的边界；要彻底封死只能让流量走服务器（见文末「硬上限方案」）。

## 文件

| 文件 | 用途 |
| --- | --- |
| `gateway.py` | 无第三方依赖的签名服务，只用 Python 标准库 |
| `xjtu-oss-gateway.service` | systemd 单元，监听 `127.0.0.1:8787` |
| `nginx-download.conf` | 限流 zone 和 `/download/` location 片段 |

## 配置

### 1. Bucket 保持私有

OSS 控制台 → Bucket `xjtu-launcher`：

- 读写权限：**私有**
- 阻止公共访问：**开启**
- 单链接限速：开启，建议 2 MB/s（防止单连接占满带宽）
- 版本控制：关闭

私有 Bucket 下直接访问对象 URL 会返回 403，只有带签名的地址能下载。

### 2. 创建最小权限 RAM 用户

RAM 控制台 → 用户 → 创建用户（只勾选「OpenAPI 调用访问」）→ 授予自定义权限：

```json
{
  "Version": "1",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": ["oss:GetObject"],
      "Resource": ["acs:oss:*:*:xjtu-launcher/*"]
    }
  ]
}
```

不要给 `PutObject`、`DeleteObject` 或 `ListObjects`。AccessKey 只放在服务器上，不进入网页、仓库或聊天记录。

### 3. 安装服务

```bash
sudo install -d /srv/xjtu-launcher/gateway /etc/xjtu-launcher
sudo install -m 755 tools/oss-gateway/gateway.py /srv/xjtu-launcher/gateway/oss-gateway.py

sudo tee /etc/xjtu-launcher/oss.env >/dev/null <<'EOF'
OSS_BUCKET=xjtu-launcher
OSS_REGION=oss-cn-beijing
OSS_ACCESS_KEY_ID=替换为RAM用户的AccessKeyId
OSS_ACCESS_KEY_SECRET=替换为RAM用户的AccessKeySecret
OSS_PUBLIC_HOST=xjtu-launcher.oss-cn-beijing.aliyuncs.com
OSS_URL_TTL=60
GATEWAY_BIND=127.0.0.1
GATEWAY_PORT=8787
SITE_ORIGIN=https://xjtu-launcher.cn
DOWNLOAD_STATS_FILE=/srv/xjtu-launcher/stats/gateway-downloads.json
# 留空则与 DOWNLOAD_STATS_FILE 同目录的 downloads.log
DOWNLOAD_LOG_FILE=
EOF

sudo chown root:www /etc/xjtu-launcher/oss.env   # Debian/Ubuntu 用 root:www-data
sudo chmod 640 /etc/xjtu-launcher/oss.env

sudo install -m 644 tools/oss-gateway/xjtu-oss-gateway.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now xjtu-oss-gateway
curl -s http://127.0.0.1:8787/healthz
```

`www` 是宝塔 / Alibaba Cloud Linux 的 nginx 用户；Debian/Ubuntu 是 `www-data`，RHEL 系也可能是 `nginx`。要同步修改 service 文件的 `User` / `Group` 以及 `oss.env` 的属组。CentOS 还需要允许 nginx 连本机端口：

```bash
sudo setsebool -P httpd_can_network_connect 1
```

### 4. nginx 限流

把 `nginx-download.conf` 里的 zone 放进 `http` 上下文（`/etc/nginx/conf.d/` 下的文件已经在 `http` 里）：

```nginx
limit_req_zone $binary_remote_addr zone=xjtu_download:10m rate=120r/m;
limit_conn_zone $binary_remote_addr zone=xjtu_download_conn:10m;
```

按 IP 的限制只是兜底：校园网和办公室 NAT 下几百个人共用一个出口 IP，真正公平的配额由网关按浏览器 Cookie 计算（见下文「按浏览器的下载配额」）。在官网的 `server` 块里加：

```nginx
location ^~ /download/ {
    limit_req zone=xjtu_download burst=60 nodelay;
    limit_conn xjtu_download_conn 50;
    limit_req_status 429;
    limit_conn_status 429;

    proxy_pass http://127.0.0.1:8787;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
}

# 更新清单每台客户端启动都会请求，不按 IP 限流；它只有几 KB，
# 一个 NAT 出口同时开机也不会互相挤掉。
location = /latest.json {
    proxy_pass http://127.0.0.1:8787;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
}
```

```bash
sudo nginx -t && sudo systemctl reload nginx
```

`rate=120r/m` 表示每个 IP 每分钟 120 次，`burst=60` 允许短时突发；`limit_conn 50` 允许同一个出口最多 50 个并发的网关请求。这个值故意放得很宽，只用来挡住明显的脚本：校园 NAT 下一个出口可能同时有几十上百人点击下载，收紧就会误伤。真正的每人配额在网关里按 Cookie 算。

`limit_conn` 统计的是网关请求，不是 OSS 文件传输；真正限制单个下载速度的是 OSS 的「单链接限速」。

### 按浏览器的下载配额

网关给第一次下载的浏览器发 `xl_dl` Cookie（随机值，`HttpOnly; SameSite=Lax`，一年有效），之后按这个 Cookie 做滑动窗口配额：默认 10 分钟内 20 次安装包下载。超出返回 `429` 和 `Retry-After`，不影响 `latest.json`、`Range` 续传和 `HEAD`。

- 环境变量：`DOWNLOAD_QUOTA`（次数，默认 20）、`DOWNLOAD_QUOTA_WINDOW`（秒，默认 600）。
- 配额在进程内存里，重启网关会清空；不需要持久化。
- 换浏览器或清 Cookie 会被当成新访客；不带 Cookie 直接请求也会各发一个新 Cookie。所以它是「让同 IP 下的正常人互不影响」的公平性措施，不是防作弊。费用兜底仍然是 OSS 单链接限速、预算告警和日志。
- 日志里记 `download_quota_exceeded`，不记 Cookie 值。

### 5. 自动上传安装包

发布时不再手动传文件。`main` 上的 `release.yml` 在草稿发布构建完成后多一个 `oss` job：

1. 从草稿 Release 下载全部资产；
2. `tools/upload-oss.mjs` 用 `ossutil` 上传到 `oss://xjtu-launcher/v<版本>/`；
3. 把 `latest.json` 六个平台的 `url` 改写成 `https://xjtu-launcher.cn/download/v<版本>/<文件名>`，再上传到 Bucket 根目录；
4. 上传成功后才把草稿变成正式 Release。

先在 GitHub 仓库设置里配置：

| 类型 | 名称 | 值 |
| --- | --- | --- |
| Variable | `OSS_BUCKET` | `xjtu-launcher` |
| Variable | `OSS_PUBLIC_BASE` | `https://xjtu-launcher.cn/download`（可留空，脚本有默认值） |
| Secret | `OSS_ACCESS_KEY_ID` | CI 专用 RAM 用户的 AccessKeyId |
| Secret | `OSS_ACCESS_KEY_SECRET` | CI 专用 RAM 用户的 AccessKeySecret |

CI 用户和网关用户要分开：网关只给 `oss:GetObject`，CI 用户再加写权限，但只限本 Bucket：

```json
{
  "Version": "1",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": ["oss:GetObject", "oss:PutObject"],
      "Resource": ["acs:oss:*:*:xjtu-launcher/*"]
    }
  ]
}
```

本地补传同一版本（`ossutil` 已配置好时）：

```bash
VERSION=0.5.2 OSS_BUCKET=xjtu-launcher OSS_CONFIG_FILE=~/.ossutilconfig \
  ASSET_DIR=/path/to/release-assets node tools/upload-oss.mjs
```

`ASSET_DIR` 里要有两个 dmg、两个 `.app.tar.gz`（含 `.sig`）、`x64-setup.exe`（含 `.sig`）和 `latest.json`。

官网的 `website/downloads.json` 仍然要随版本更新 `version`、`date`、`base_url`、文件大小与 SHA256；它不参与上面的自动改写，避免工作流回写 `main`。

### 6. 改官网下载地址

`website/downloads.json`：

```json
"base_url": "https://xjtu-launcher.cn/download/v0.5.2/"
```

重新部署 `website/`。确认页面的下载按钮指向 `/download/...`，不是 `aliyuncs.com`。

## 下载计数

设置 `DOWNLOAD_STATS_FILE`（要与 site-api 的 `STATS_DIR` 同一个目录）后，网关会把每次**不带 `Range` 的安装包 GET** 记成一次下载开始，写入该 JSON：`{"total": n, "files": {"v0.5.2/xxx.dmg": n}, "updated_at": 时间戳}`。`.sig` 与 `latest.json` 不计；Range / 续传不重复计。写入失败只记日志，不影响下载。

site-api 读取这个文件的 `total`，加到 GitHub Releases API 的历史下载量上；安装包迁到 OSS 后 GitHub 的计数不再增长，页面上的数字靠这部分继续增加。它是「下载开始」的次数，不是完成数。

### 日志

网关每次计数都往 `downloads.log` 追加一行 JSON（默认与计数文件同目录，可用 `DOWNLOAD_LOG_FILE` 改）：

```json
{"t":"2026-11-08T00:35:19Z","event":"download","key":"v0.5.2/xxx.dmg","total":12,"file_total":7}
```

还记录 `manifest`（每次 `latest.json` 请求）、`referer_rejected`、`bad_key`、`download_count_failed` 和服务启动。日志不含 Referer 原文、请求头或 AccessKey；Referer 可能被伪造且可能带个人路径，只记拒绝事实。超过 5 MB 轮转成 `.1`，保留 5 个旧文件。计数本身每次下载立即原子写入，日志只是审计，写日志失败不影响下载。

site-api 的定时快照会把 `gateway-downloads.json` 一起复制到 `/srv/xjtu-launcher/stats/backups/`，见 `tools/site-api/README.md`。

## 验证

```bash
# 1. 未签名直连 OSS 应返回 403
curl -sI https://xjtu-launcher.oss-cn-beijing.aliyuncs.com/v0.5.2/XJTU.Launcher_0.5.2_aarch64.dmg

# 2. 网关应返回 302，Location 是带签名的 OSS 地址
curl -sI https://xjtu-launcher.cn/download/v0.5.2/XJTU.Launcher_0.5.2_aarch64.dmg

# 3. 跟随跳转应能下载
curl -sIL https://xjtu-launcher.cn/download/v0.5.2/XJTU.Launcher_0.5.2_aarch64.dmg | head

# 4. 连发 20 次，应出现 429
for i in $(seq 1 20); do
  curl -s -o /dev/null -w "%{http_code}\n" \
    https://xjtu-launcher.cn/download/v0.5.2/XJTU.Launcher_0.5.2_aarch64.dmg
done
```

第 4 步会消耗配额，等一分钟再测真实下载。Tauri 更新器会跟随 302；用真实旧版客户端验收时确认能完成「检查更新 → 下载 → 校验」。

## 费用兜底

限流能挡住脚本，但挡不住大量不同 IP。再加三层：

1. **OSS 单链接限速**：2 MB/s，单个连接刷不快。
2. **费用预算告警**：费用中心 → 预算管理，按月设置上限（例如 ¥50），在 50% / 80% / 100% 发通知。
3. **OSS 日志 + 监控报警**：开启访问日志，观察外网流出流量和前几名 IP；异常时在 WAF 或安全组临时封禁。

阿里云没有「超过预算自动停机」的硬开关，告警需要有人看。如果要求账单绝对不超，用下面的方案。

## 硬上限方案（可选）

如果更在意「账单不可能爆」而不是下载速度，可以让文件也走服务器：

- 签名服务返回 OSS **内网地址** `https://xjtu-launcher.oss-cn-beijing-internal.aliyuncs.com/...`。
- nginx 用 `proxy_pass` 拉取并把字节转发给用户，同时加 `limit_rate 512k`。
- 同地域 ECS 访问 OSS 内网不计外网流量费，成本只取决于服务器固定带宽。

代价是服务器出口只有约 3.4 Mbps：一个 25 MB 的包约一分钟，多人同时下载会互相拖慢。官网页面和签名服务仍然很轻，不受影响。

## 注意

- 不要把 AccessKey 写进 `downloads.json`、网页脚本或仓库；签名只在服务器上做。
- 签名链接 60 秒过期；泄露的链接只能再用很短时间。
- `SITE_ORIGIN` 的 Referer 检查只是减少外站盗链，允许空 Referer（桌面更新器不发 Referer），不能当作鉴权。备案前 IP 预览也要下载时，用逗号分隔临时加上 `http://8.152.100.169`，域名上线后删掉这一项。
- 服务只监听 `127.0.0.1`，不要改成 `0.0.0.0`。
- 更换 Bucket 或接入自定义下载域名时，同步改 `OSS_PUBLIC_HOST` 和 `downloads.json` 的 `base_url`。
