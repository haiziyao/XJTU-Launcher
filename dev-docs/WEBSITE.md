# 官网与下载分发

更新于 2026-11-08。记录官网源码、部署方式、对象存储（OSS）放安装包的流程，以及后续把更新源切到 HTTPS 的计划。

## 现状

- 域名：`xjtu-launcher.cn`，2026-11-08 在阿里云（万网）注册，用户已添加 A 记录指向 `8.152.100.169`。
- 2026-11-08 04:00 `whois` 为 `clientHold`，04:06 已变为 `ok`；`223.5.5.5` 与 `8.8.8.8` 已解析到 `8.152.100.169`，`119.29.29.29` 当时仍未刷新。80 端口已到 nginx（默认站点返回「没有找到站点」），443 未监听。
- ICP 备案：**2026-11-08 实测域名被阿里云拦截**，`http://xjtu-launcher.cn/` 返回 403，页面标题 `Non-compliance ICP Filing`（响应头 `Server: Beaver`）。新注册的域名需要单独在阿里云备案系统提交「新增网站」，已有的备案主体或旧域名不能直接覆盖它；服务器在阿里云北京，备案完成前 80/443 都不能对外提供网站服务。**备案是官网域名上线的唯一阻塞项。**
- 服务器：2026-11-08 已用 root 密码登录并安装本机公钥（`~/.ssh/id_ed25519`），之后免密登录。系统是 Alibaba Cloud Linux 3（宝塔面板管理 nginx 1.24，`www` 用户），Python 3.6.8。
- 官网源码：`website/`，纯静态 HTML/CSS/JS，没有构建步骤，没有外部字体、脚本或统计代码。
- 部署：2026-11-08 已上传到 `/srv/xjtu-launcher/site`，nginx vhost 为 `/www/server/panel/vhost/nginx/xjtu-launcher.cn.conf`。备案完成前可用 `http://8.152.100.169/site/` 预览。
- 2026-11-08 改版：按用户要求换成白底 + 轮播图 + 下载卡片的版式，已部署到 `/srv/xjtu-launcher/site`。第二轮按反馈改成居中 coverflow：中间一张完整、左右两张缩小倾斜并模糊内容（保留清晰窗口边框）、自动轮播 4.6s，首屏在 1280×720 / 1280×800 / 1366×768 / 1440×900 都能同时看到标题、轮播和三张下载卡。下载卡按当前设备高亮并排到最前（macOS 架构优先读 UA-CH，其次 WebGL renderer，取不到按 Apple 芯片处理）。
- `docs/design/website-v1/C-carousel/` 是上一版（非 coverflow）的参考稿，部署版本以 `website/` 为准。**尚未经过用户验收；域名仍被 ICP 拦截，正式上线未完成。**
- 安装包目前指向 GitHub Release。OSS Bucket `xjtu-launcher` 已创建，但尚未上传安装包、尚未创建 RAM 用户；`downloads.json` 暂时指向 GitHub Release。
- 下载网关：`tools/oss-gateway/` 已实现，签名结果与官方 `ali-oss` SDK 一致；2026-11-08 已安装到 `/srv/xjtu-launcher/gateway` 并注册 systemd 单元，**因缺少 AccessKey 仍未启动**。更新清单的自动上传见 `release.yml` 的 `oss` job 与 `tools/upload-oss.mjs`，需先配置 GitHub 变量与 Secret。

## 目录

```text
website/
├─ index.html        单页官网（白底、轮播、下载卡片）
├─ styles.css        样式（纯白、系统字体、无外部资源）
├─ app.js            读取 downloads.json，填充下载卡片、版本行、SHA256 明细与当前设备提示
├─ carousel.js       轮播：自动播放、进度点、键盘、触摸滑动、指针视差、`#slide-N` 深链
├─ downloads.json    版本、日期、下载基址、文件名、大小、SHA256
├─ assets/           图标与截图（由 docs/screenshots 和 src-tauri/icons 生成）
├─ robots.txt
└─ sitemap.xml
```

## 更新下载信息

每次发版后改 `website/downloads.json`：

- `version`、`date`：与 `docs/releases/v<版本>.md` 一致。
- `base_url`：安装包所在目录，末尾带 `/`。OSS 启用后改成 `https://<bucket>.oss-cn-beijing.aliyuncs.com/v<版本>/`。
- `files[]`：只列 macOS arm64、macOS x64、Windows x64 三个安装包；`size` 用字节，`sha256` 从 GitHub Release 资产摘要或 `shasum -a 256` 取。

页面的三张下载卡片按 `platform` + `arch` 匹配，自动填链接、大小与 SHA256；探测到当前设备时对应的卡片会高亮（macOS 架构靠 WebGL renderer 判断，取不到就不高亮）。`downloads.json` 读取失败时卡片回退到 GitHub Release。

设计参考稿在 `docs/design/website-v1/C-carousel/`；正式页面以 `website/` 为准。

## 部署到服务器

### 1. 上传文件

服务器没有 `rsync`，本机 tar 通过 SSH 传：

```bash
COPYFILE_DISABLE=1 tar -C website --exclude README.md -czf - . |
  ssh root@8.152.100.169 'rm -rf /srv/xjtu-launcher/site && \
  mkdir -p /srv/xjtu-launcher/site && tar -xzf - -C /srv/xjtu-launcher/site && \
  chown -R www:www /srv/xjtu-launcher/site'
```

`COPYFILE_DISABLE=1` 防止 macOS 生成 `._*` AppleDouble 文件；忘了加就在服务器上 `find ... -name '._*' -delete`。

### 2. nginx 站点

这台服务器由宝塔管理，vhost 放在 `/www/server/panel/vhost/nginx/`，主配置会自动 `include *.conf`。当前配置：

- 站点：`/www/server/panel/vhost/nginx/xjtu-launcher.cn.conf`（80，`xjtu-launcher.cn` 与 `www`）。
- 限流 zone：`/www/server/nginx/conf/xjtu-launcher-limits.conf`，在 `nginx.conf` 的 `http` 块里 include。
- 预览入口：`8.152.100.169_80.conf` 里的 `location ^~ /site/`，用于备案前查看官网。
- 原有 `location ^~ /xjtu-launcher/`（旧 HTTP 更新镜像）保持不变，P0 迁移完成前不能删。

```bash
nginx -t && nginx -s reload
```

宝塔面板如果重新生成 nginx 配置，要确认 `http` 块里的 limits include 仍然存在，否则 `nginx -t` 会报 `zero size shared memory zone "xjtu_download"`。

### 3. HTTPS

**备案完成前无法申请**：Let's Encrypt 的 HTTP-01 验证请求会被阿里云备案拦截，拿不到证书；443 目前也未监听。备案通过后：

```bash
dnf install -y certbot python3-certbot-nginx   # 或宝塔面板 → SSL → Let's Encrypt
certbot --nginx -d xjtu-launcher.cn -d www.xjtu-launcher.cn
```

如果 80 端口仍被拦，改用 DNS-01（acme.sh + 阿里云 DNS API，需要额外的 DNS 权限 AccessKey）。

### 4. 验证

备案前只能在服务器本机带 `Host` 头验证：

```bash
curl -I -H 'Host: xjtu-launcher.cn' http://127.0.0.1/
curl -s -H 'Host: xjtu-launcher.cn' http://127.0.0.1/downloads.json | head -c 200
curl -I -H 'Host: xjtu-launcher.cn' http://127.0.0.1/assets/home.jpg
```

公网预览（不需要备案，用 IP 访问）：`http://8.152.100.169/site/`。备案通过后换成：

```bash
dig +short xjtu-launcher.cn
curl -I https://xjtu-launcher.cn/
```

页面应返回 `200`，`downloads.json` 返回 `application/json`，证书链有效。

## 用 OSS 存安装包（私有 Bucket + 下载网关）

服务器出口带宽约 3.4 Mbps，安装包不适合放在服务器上；但 OSS 也不能开公共读。公共读的对象 URL 固定，被爬走后可以无限下载，外网流出流量没有硬性上限。做法是：**Bucket 私有，官网的下载链接指向服务器，由服务器签发 60 秒有效的 OSS 地址，并按 IP 限流**。完整安装与验证步骤见 [`tools/oss-gateway/README.md`](../tools/oss-gateway/README.md)。

### 请求流程

```text
浏览器 / 应用
  → https://xjtu-launcher.cn/download/v0.5.2/XJTU.Launcher_0.5.2_aarch64.dmg
  → nginx：每个 IP 12 次/分钟、最多 4 个并发，超出 429
  → 127.0.0.1:8787 校验路径，签发 60 秒有效的 OSS 地址
  → 302 到 https://xjtu-launcher.oss-cn-beijing.aliyuncs.com/...
  → OSS 直接把安装包发给用户
```

限流只作用于「签发」。签名链接在 60 秒内可以被重复使用，这是方案的边界；要彻底避免外网流量费，只能让文件也走服务器（见文末）。

### Bucket 设置

Bucket `xjtu-launcher`（已创建），地域华北 2（北京）：

- 读写权限：**私有**
- 阻止公共访问：**开启**
- 单链接限速：开启，建议 2 MB/s
- 版本控制、服务端加密：关闭

默认地址 `https://xjtu-launcher.oss-cn-beijing.aliyuncs.com/` 支持 HTTPS。私有 Bucket 直接访问对象返回 403，只有带签名的地址能下载。

### 下载网关

`tools/oss-gateway/` 是 Python 标准库实现的签名服务，不引入第三方包：

- `gateway.py` 监听 `127.0.0.1:8787`，只接受 `v<版本>/<文件名>` 与 `latest.json`；兼容服务器上的 Python 3.6（`ThreadingHTTPServer` 在 3.7 才有，代码里自带等价实现）。
- `xjtu-oss-gateway.service` 负责开机启动，以 `www` 运行，路径 `/srv/xjtu-launcher/gateway/oss-gateway.py`。
- `nginx-download.conf` 声明限流 zone、`/download/` 与 `/latest.json` 反向代理。

AccessKey 使用只有 `oss:GetObject` 权限的 RAM 用户，保存在 `/etc/xjtu-launcher/oss.env`（`600 root:root`），不进入网页、仓库或聊天记录。2026-11-08 已安装文件与服务，`oss.env` 里 AccessKey 仍是空值，服务处于 `disabled`，填入后执行：

```bash
systemctl enable --now xjtu-oss-gateway
curl -s http://127.0.0.1:8787/healthz
```

安装包不再手动上传：`release.yml` 的 `oss` job 调用 `tools/upload-oss.mjs`，把草稿资产传到 `oss://xjtu-launcher/v<版本>/` 并把 `latest.json` 的六个平台地址改写成网关地址。需要先配置 GitHub 变量 `OSS_BUCKET`、`OSS_PUBLIC_BASE` 与 Secret `OSS_ACCESS_KEY_ID`、`OSS_ACCESS_KEY_SECRET`，见 [`tools/oss-gateway/README.md`](../tools/oss-gateway/README.md)。

### nginx 限流

```nginx
limit_req_zone $binary_remote_addr zone=xjtu_download:10m rate=12r/m;
limit_conn_zone $binary_remote_addr zone=xjtu_download_conn:10m;

location ^~ /download/ {
    limit_req zone=xjtu_download burst=6 nodelay;
    limit_conn xjtu_download_conn 4;
    limit_req_status 429;
    limit_conn_status 429;
    proxy_pass http://127.0.0.1:8787;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
}
```

正常用户点三个下载按钮远低于 12 次/分钟；更新检查只发一次请求。签名服务所在目录与 `downloads.json` 的 `base_url` 要一起更新：

```json
"base_url": "https://xjtu-launcher.cn/download/v0.5.2/"
```

### 费用兜底

- 单链接限速 2 MB/s，单个连接刷不快。
- 费用中心 → 预算管理，按月设置上限（例如 ¥50），在 50% / 80% / 100% 发通知。
- 开启 OSS 访问日志，观察外网流出流量和前几名 IP；异常时用 WAF 或安全组临时封禁。
- 阿里云没有「超过预算自动停机」的硬开关，告警需要有人处理。

### 硬上限方案（可选）

如果更在意账单不可能超，而不是下载速度：

- 签名服务返回 OSS **内网地址** `https://xjtu-launcher.oss-cn-beijing-internal.aliyuncs.com/...`。
- nginx 用 `proxy_pass` 转发字节，并加 `limit_rate 512k`。
- 同地域 ECS 访问 OSS 内网不计外网流量费，成本只取决于服务器固定带宽。

代价是 3.4 Mbps 出口：一个 25 MB 的包约一分钟，多人同时下载会互相拖慢。这个模式下 OSS 只承担存储，不再承担带宽。

## 后续：更新源切到 HTTPS

安装包放 OSS 后，`latest.json` 也放私有 Bucket，由官网统一入口提供：

```text
https://xjtu-launcher.cn/latest.json
```

清单里的 `platforms[*].url` 指向 `https://xjtu-launcher.cn/download/v<版本>/<文件>`，由下载网关 302 到 OSS；`signature` 保持 Tauri 生成的值不变。Tauri 更新器使用 reqwest，默认跟随重定向，但必须用真实旧版客户端验证一次。然后按 [P0.md](P0.md) 的退出条件改代码并发一个新版本：

1. `src-tauri/src/updates.rs` 的 `DEFAULT_ENDPOINT`。
2. `.github/workflows/release.yml` 的 `XJTU_UPDATE_ENDPOINT`。
3. 恢复 `config.rs`、`AppUpdateSettings.vue`、`prepare-release.mjs` 的 HTTPS-only 校验。
4. 发版后用真实旧版客户端走通「检查更新 → 下载 → 校验 → 安装 → 重启」。

客户端内置地址是编译进去的，老用户必须装到新版本才会切换；旧 HTTP 镜像在迁移完成前继续保留。

## 验收状态

- 官网页面：已实现，本机用 Chrome 在 1440 px 和 390 px 宽度下检查过布局，没有横向溢出。
- 域名解析：whois 已恢复 `ok`，`223.5.5.5` / `8.8.8.8` 解析到 `8.152.100.169`；`www` 尚未添加记录。
- 部署：2026-11-08 已完成。`http://8.152.100.169/site/` 返回 200，`styles.css` / `app.js` / `downloads.json` / 截图均 200；带 `Host: xjtu-launcher.cn` 访问也返回 200。
- ICP 备案：**未完成**，域名被阿里云拦截（403 `Non-compliance ICP Filing`），这是域名上线的唯一阻塞项。
- HTTPS：**未完成**，443 未监听；备案前无法完成 HTTP-01 验证。
- OSS：Bucket `xjtu-launcher` 已创建，未上传安装包、未创建 RAM 用户；`downloads.json` 暂时指向 GitHub Release。
- 下载网关：代码、本机测试与服务器安装已完成；**未启动**（缺 AccessKey），限流与 302 尚未在真实链路验证。
- 自动上传：`tools/upload-oss.mjs` 与 `release.yml` 的 `oss` job 已就绪，用假 `ossutil` 做过一次全流程试跑（9 个文件、`latest.json` 六个平台 URL 改写正确），**未经真实 OSS 验证**。
- 官网内容按 0.5.2 当前能力编写；0.6 功能发布前不要提前写进页面。
