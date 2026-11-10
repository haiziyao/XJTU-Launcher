# 官网与下载分发

更新于 2026-11-08。记录官网源码、部署方式、对象存储（OSS）放安装包的流程，以及后续把更新源切到 HTTPS 的计划。

## 现状

- 域名：`xjtu-launcher.cn`，2026-11-08 在阿里云（万网）注册，`whois` 状态为 `ok`，到期 2027-11-08。
- 2026-11-08 04:00 `whois` 曾为 `clientHold`，04:06 变为 `ok`，当时 `223.5.5.5` / `8.8.8.8` 短暂解析到 `8.152.100.169`。**备案期间阿里云不允许域名解析**：11 月 8 日晚复查，权威 NS（`dns23/dns24.hichina.com`）已无 A 记录，多个公共 DNS 均返回空，`www` 也从未添加。因此官网和下载暂时只走 `http://8.152.100.169/`（用户 2026-11-08 决定：备案期间先用 IP，不再尝试解析）。
- ICP 备案：**2026-11-08 实测域名被阿里云拦截**，直接带 `Host: xjtu-launcher.cn` 请求服务器 IP 也返回 403，页面标题 `Non-compliance ICP Filing`（响应头 `Server: Beaver`）。新注册的域名需要单独在阿里云备案系统提交「新增网站」，已有的备案主体或旧域名不能直接覆盖它；服务器在阿里云北京，备案完成前 80/443 都不能对外提供网站服务。**备案是官网域名上线的唯一阻塞项。**
- 服务器：2026-11-08 已配置 SSH 免密登录，登录凭据只保存在本机，不写入仓库或共享文档。系统是 Alibaba Cloud Linux 3（宝塔面板管理 nginx 1.24，`www` 用户），Python 3.6.8。
- 官网源码：`website/`，纯静态 HTML/CSS/JS，没有构建步骤，没有外部字体、脚本或统计代码。
- 部署：2026-11-08 已上传到 `/srv/xjtu-launcher/site`，nginx vhost 为 `/www/server/panel/vhost/nginx/xjtu-launcher.cn.conf`。备案完成前可用 `http://8.152.100.169/site/` 预览。
- 2026-11-08 改版：按用户要求换成白底 + 轮播图 + 下载卡片的版式，已部署到 `/srv/xjtu-launcher/site`。第二轮按反馈改成居中 coverflow：中间一张完整、左右两张缩小倾斜并模糊内容（保留清晰窗口边框）、自动轮播 4.6s，首屏在 1280×720 / 1280×800 / 1366×768 / 1440×900 都能同时看到标题、轮播和三张下载卡。下载卡按当前设备高亮并排到最前（macOS 架构优先读 UA-CH，其次 WebGL renderer，取不到按 Apple 芯片处理）。
- 2026-11-08 第三轮改版：去掉首屏淡淡的彩色底色；大标题改成逐字蹦出的打字效果（`prefers-reduced-motion` 时不播放，`aria-label` 保留完整句子）；原来「MIT 开源」的胶囊换成一排标签（版本号、发布阶段、Rust、Tauri 2、Vue 3、MIT）。发布阶段标签由 `downloads.json` 的主版本号决定：`0.x` 显示「预览版」，`1.x` 起显示「正式版」。用户决定不做校园照片背景。标签的发布阶段类名用 `tag-stage` 而不是 `stage`，因为 `.stage` 已被轮播容器占用（同名时会套上轮播的 `height` 与 `perspective`）。
- 2026-11-08 第四轮改版（用户提出）：标签改成小圆角、每个标签一种颜色（含小圆点），不再用胶囊；下载区下面删掉原来的「功能」三卡，换成三栏「方向」——功能更强大 / 社区更丰富 / 页面更美观，各带占位示意图与一个点赞按钮，点赞数作为下一步优先级参考；三栏下面是「关于这个项目」（项目、团队、理念、畅想）；导航新增 Q&amp;A，指向 `doc/qa/`；页面右上角（导航栏）显示安装包下载次数。占位示意图是手画的 SVG，角上标注「示意图」，有真实配图后替换。
- 2026-11-08 第五轮改版（用户反馈）：标签去掉小圆点，改成方角、加粗字号，版本标签深底白字、发布阶段标签琥珀色、技术标签用等宽字体；下载量从导航栏移到顶栏下方、首屏右上角；下载卡片改成玻璃质感（半透明 + `backdrop-filter`），推荐卡不透明度更高并保留紫色描边，不再用黑色卡片；Q&amp;A 页面改成左侧目录（分类 + 问题两级，默认高亮「常见问题」），点击目录打开对应问题并滚动定位，滚动时跟随高亮，窄屏下目录收成可展开的「目录」面板（`doc/qa/qa.js`）。顶栏导航的 `nav` 选择器改成 `.top nav`，避免侧边目录继承导航的 flex 布局。
- 2026-11-08 第六轮改版（用户提出）：导航「方向」左侧新增「社区广场」，指向 `community/`。最终版是白底、浅色、分离的三条胶囊式竖带：深紫 / 深青 / 深琥珀只作为文字色和极浅底色，没有实心色块、渐变、阴影和发光；宋体竖排「更丰富 / 更便捷 / 更自由」就是标题，XJTU 是下方居中的黑色 Didot 斜体。深色霓虹版、首尾相接的深色块、彩色渐变 XJTU 都已被用户否掉，不要再改回去。载入时三条带逐条向上揭开，之后不同相位浮动，悬停轻微放大；`prefers-reduced-motion` 下停止。第二屏白底卡片说明之后对接应用内社区，只链到方向与 Q&amp;A。窄屏导航不再用 `nth-child(3)` 隐藏链接（会误伤「下载」），改为 `data-sm="hide"`。
- `docs/design/website-v1/C-carousel/` 是上一版（非 coverflow）的参考稿，部署版本以 `website/` 为准。**尚未经过用户验收；域名仍被 ICP 拦截，正式上线未完成。**
- 安装包已切到 OSS：2026-11-08 把 v0.5.2 的 9 个发布资产上传到私有 Bucket `xjtu-launcher` 的 `v0.5.2/`，`latest.json` 上传到 Bucket 根目录；`downloads.json` 的 `base_url` 改为 `/download/v0.5.2/`，页面的三个下载按钮都经过下载网关。
- 下载网关：`tools/oss-gateway/` 已安装到 `/srv/xjtu-launcher/gateway` 并已 `enable --now`，监听 `127.0.0.1:8787`。2026-11-08 创建了两个最小权限 RAM 用户：`xjtu-launcher-gateway` 只有 `oss:GetObject`（密钥在 `/etc/xjtu-launcher/oss.env`，`640 root:www`），`xjtu-launcher-ci` 有 `GetObject + PutObject`（只放在 GitHub Secrets 和本机 ossutil 配置里）。已验证：匿名直连 OSS 403、网关签名 302、跟随跳转 206、伪造 Referer 403、网关密钥 PutObject 403、同 IP 连发第 7 次起 429。
- 备案前 IP 预览也能下载：`8.152.100.169_80.conf` 里临时加了 `/download/` 与 `= /latest.json` 反代，`oss.env` 的 `SITE_ORIGIN` 临时包含 `http://8.152.100.169`。域名上线后删掉这两处临时配置。
- 自动上传：GitHub 变量 `OSS_BUCKET=xjtu-launcher`、`OSS_PUBLIC_BASE=https://xjtu-launcher.cn/download` 与 Secret `OSS_ACCESS_KEY_ID` / `OSS_ACCESS_KEY_SECRET` 已配置，`release.yml` 的 `oss` job 从下一版起会自动上传。

## 目录

```text
website/
├─ index.html        首页（白底、轮播、下载卡片、三栏方向、项目介绍、安装）
├─ styles.css        样式（纯白、系统字体、无外部资源）
├─ app.js            读取 downloads.json 与 /api/stats，填充下载卡片、版本行、发布阶段标签、下载量、点赞数、SHA256 明细与当前设备提示；大标题逐字打字
├─ carousel.js       轮播：自动播放、进度点、左右箭头、键盘、触摸滑动、`#slide-N` 深链
├─ downloads.json    版本、日期、下载基址、文件名、大小、SHA256
├─ assets/           图标、截图，以及三张方向占位图 direction-*.svg
├─ community/index.html 社区广场（图标式首屏，之后对接社区内容）
├─ doc/qa/index.html Q&A 页面（左侧目录 + 分类问题，对应 /doc/qa/）
├─ doc/qa/qa.js      Q&A 目录：默认高亮、点击打开并定位、滚动跟随、窄屏折叠
├─ robots.txt
└─ sitemap.xml
```

## 更新下载信息

**发版不再手改官网版本信息。** `downloads.json` 由发版流水线生成并上传到 Bucket 根目录，官网从同一个地址读取：

- 生成：`tools/site-manifest.mjs` 读三个安装包，算出 `size` 与 `sha256`，写出 `{version, date, releases_url, base_url, files[]}`；`tools/upload-oss.mjs` 在上传资产之后调用它，上传到 `oss://xjtu-launcher/downloads.json`（不在 `v<版本>/` 里）。`date` 取 `latest.json` 的 `pub_date`。
- 分发：nginx 的 `= /downloads.json`（域名）与 `= /site/downloads.json`（IP 预览）反代到网关；网关 `downloads.json` 分支**直接回 JSON 字节**而不是 302，长度上限 256 KB，JSON 解析失败回 502。这样桶里没有对象时 nginx 的 `proxy_intercept_errors` 能接住 404，退回站内静态副本（`error_page 404 = /_downloads_static`，`default_type application/json`，`internal`）。
- 消费：首页 `app.js` 与 Q&amp;A 的 `qa.js` 都读这个文件。`doc/qa/index.html` 里两处 `[data-qa-version]` / `[data-qa-date]` 是占位符，读到清单才填，读不到就保持 `v—`，不显示没人核对过的版本号。
- 兜底：`website/downloads.json` 只是桶里没有对象时的静态副本，仍随站点发布，**不要**把它当成唯一数据源；补发旧版本时手工更新它或跑一次 `Site manifest` 工作流。
- 手动补：`.github/workflows/site-manifest.yml`（`Site manifest`，`workflow_dispatch`，输入版本号）会从已发布 Release 下载三个安装包、重建并上传 `downloads.json`，用于这一步上线前发布的版本（目前是 0.6.6）。

字段约定：`base_url` 用站内绝对路径 `/download/v<版本>/`，由 nginx 转给下载网关，签名后 302 到私有 OSS，同一份配置在 IP 预览和正式域名下都能用；`files[]` 只列 macOS arm64、macOS x64、Windows x64 三个安装包，`size` 用字节，`sha256` 取自资产本身（流水线现算）。

页面的三张下载卡片按 `platform` + `arch` 匹配，自动填链接、大小与 SHA256；探测到当前设备时对应的卡片会高亮（macOS 架构靠 WebGL renderer 判断，取不到就不高亮）。`downloads.json` 读取失败时卡片回退到 GitHub Release。

设计参考稿在 `docs/design/website-v1/C-carousel/`；正式页面以 `website/` 为准。

## 点赞与下载量接口

首页三张方向卡片的点赞数、首屏右上角（顶栏下方）的下载量来自 `tools/site-api/`（安装与隐私细节见该目录的 README）：

- 服务：`site-api.py`，只依赖 Python 3.6 标准库，监听 `127.0.0.1:8788`；systemd 单元 `xjtu-site-api.service`，以 `www` 运行，状态目录 `/srv/xjtu-launcher/stats`。
- 路由：正式域名的 `/api/`、备案前 IP 预览的 `/site/api/` 都反代到同一服务；nginx 每 IP 30 次/分钟，超出返回 429。
- `GET /stats` 返回 `{"downloads": {"total": 46}, "likes": {...}, "voted": {"powerful": false, ...}}`，`voted` 按当前浏览器 Cookie 计算；`POST /like` 的 body 是 `{"key": "powerful|community|design"}`，返回 `{"count": n, "voted": true/false}`，同一浏览器再点一次即取消。
- 点赞存在 `likes.json`；投票按第一方 Cookie `xl_voter` 去重（同一浏览器对同一方向一票，再点一次取消），不按 IP，避免校园网 NAT 下不同的人被算成一票。Cookie 值只保存加盐 SHA256 摘要，不落盘原始值，也不写日志。
- 下载量 = GitHub Releases API 的历史计数 + OSS 下载网关的计数。GitHub 部分按安装包后缀（`.dmg` `.exe` `.msi` `.AppImage` `.deb`）累加，不统计 `latest.json` 和签名文件；缓存 600 秒。安装包改由 OSS 提供后 GitHub 不再增长，增长来自 `tools/oss-gateway/gateway.py` 写入的 `/srv/xjtu-launcher/stats/gateway-downloads.json`：只记不带 `Range` 的安装包 `GET`（下载开始次数，不是完成数），`HEAD`、续传、`latest.json`、`.sig` 不计。
- GitHub 不可达时沿用上一次真实值；两边都没有值时接口返回 `null`，页面隐藏下载量，不显示编造的数字。网关计数从功能上线起算，不包含之前的历史下载。
- 前端不再用 localStorage 记投票状态：`/stats` 的 `voted` 字段按浏览器 Cookie 返回「我投过哪些方向」，页面加载后以服务端为准。总票数永远显示为「N 人支持」，自己投过时按钮变红并显示「N 人支持 · 你已投」，陌生人看到的是灰底「N 人支持」，不会把别人的票误认成自己的。
- 按钮是开关：再点一次「你已投」会取消投票，服务端票数减一并返回 `voted: false`，标签恢复「人支持」。接口失败时按钮显示「暂时不可用」，不会假装成功。测试投票写进真实计数，验证后执行 `rm -f /srv/xjtu-launcher/stats/likes.json && systemctl restart xjtu-site-api` 清掉；测试下载量清 `gateway-downloads.json`（网关每次请求现读，不用重启）。
- 日志：`stats/likes.log` 每次投票一行（方向、累计票数、加盐哈希前 8 位；不含原始 IP），`stats/downloads.log` 每次计入的下载、更新检查（`manifest`）、拒绝的 Referer 与非法路径各一行，`stats/stats.log` 记服务启动、GitHub 抓取与快照。三个文件都按 5 MB 轮转、各留 5 个旧文件。
- 备份：site-api 每 900 秒把 `likes.json` 与 `gateway-downloads.json` 复制到 `stats/backups/`，各保留最近 192 份（约两天）；点赞每次投票本来就已原子落盘，快照是额外兜底。份数与间隔可用 `STATS_SNAPSHOT_KEEP` / `STATS_SNAPSHOT_INTERVAL` 调整。

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

服务器出口带宽约 3.4 Mbps，安装包不适合放在服务器上；但 OSS 也不能开公共读。公共读的对象 URL 固定，被爬走后可以无限下载，外网流出流量没有硬性上限。做法是：**Bucket 私有，官网的下载链接指向服务器，由服务器签发 60 秒有效的 OSS 地址；每人配额按浏览器 Cookie 算，nginx 按 IP 的限流只做宽松兜底**。完整安装与验证步骤见 [`tools/oss-gateway/README.md`](../../tools/oss-gateway/README.md)。

### 请求流程

```text
浏览器 / 应用
  → https://xjtu-launcher.cn/download/v0.5.2/XJTU.Launcher_0.5.2_aarch64.dmg
  → nginx：每个 IP 120 次/分钟、最多 50 个并发（只挡脚本，不区分 NAT 下的个人）
  → 127.0.0.1:8787 校验路径与浏览器 Cookie 配额（默认 20 次/10 分钟），签发 60 秒有效的 OSS 地址
  → 302 到 https://xjtu-launcher.oss-cn-beijing.aliyuncs.com/...
  → OSS 直接把安装包发给用户
```

`latest.json` 是每台客户端启动都会请求的更新清单，不设按 IP 限流。限流只作用于「签发」，签名链接在 60 秒内可以被重复使用，这是方案的边界；费用兜底是 OSS 单链接限速、预算告警和访问日志，要彻底避免外网流量费只能让文件也走服务器（见文末）。

### Bucket 设置

Bucket `xjtu-launcher`（已创建），地域华北 2（北京）：

- 读写权限：**私有**
- 阻止公共访问：**开启**
- 单链接限速：开启，建议 2 MB/s
- 版本控制、服务端加密：关闭

默认地址 `https://xjtu-launcher.oss-cn-beijing.aliyuncs.com/` 支持 HTTPS。私有 Bucket 直接访问对象返回 403，只有带签名的地址能下载。

### 下载网关

`tools/oss-gateway/` 是 Python 标准库实现的签名服务，不引入第三方包：

- `gateway.py` 监听 `127.0.0.1:8787`，只接受 `v<版本>/<文件名>`、`latest.json` 与 `downloads.json`；前两者 302 到签名地址，`downloads.json` 由网关取回字节直接返回（详见「更新下载信息」）。兼容服务器上的 Python 3.6（`ThreadingHTTPServer` 在 3.7 才有，代码里自带等价实现）。
- `xjtu-oss-gateway.service` 负责开机启动，以 `www` 运行，路径 `/srv/xjtu-launcher/gateway/oss-gateway.py`。
- `nginx-download.conf` 声明限流 zone、`/download/` 与 `/latest.json` 反向代理；两个 vhost 另有 `= /downloads.json` 与 `= /site/downloads.json` 精确匹配，带 404 退回静态副本。

AccessKey 使用只有 `oss:GetObject` 权限的 RAM 用户，保存在 `/etc/xjtu-launcher/oss.env`（`640 root:www`，服务以 `www` 读取），不进入网页或仓库。2026-11-08 已填好密钥并执行：

```bash
systemctl enable --now xjtu-oss-gateway
curl -s http://127.0.0.1:8787/healthz   # 返回 ok
```

`SITE_ORIGIN` 支持逗号分隔的多个来源；备案前临时是 `https://xjtu-launcher.cn,http://8.152.100.169`，域名上线后删掉 IP 那一项。

安装包不再手动上传：`release.yml` 的 `oss` job 调用 `tools/upload-oss.mjs`，把草稿资产传到 `oss://xjtu-launcher/v<版本>/` 并把 `latest.json` 的六个平台地址改写成网关地址。GitHub 变量 `OSS_BUCKET`、`OSS_PUBLIC_BASE` 与 Secret `OSS_ACCESS_KEY_ID`、`OSS_ACCESS_KEY_SECRET` 已于 2026-11-08 配置，见 [`tools/oss-gateway/README.md`](../../tools/oss-gateway/README.md)。

### 限流与配额

```nginx
limit_req_zone $binary_remote_addr zone=xjtu_download:10m rate=120r/m;
limit_conn_zone $binary_remote_addr zone=xjtu_download_conn:10m;

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
```

校园网和办公室 NAT 下很多人共用一个出口 IP，所以按 IP 的 120 次/分钟只是兜底；真正公平的是网关里的浏览器 Cookie 配额：第一次下载签发 `xl_dl`，默认 10 分钟 20 次，超出网关返回 429 并带 `Retry-After`，换浏览器或清 Cookie 会重新计算。`latest.json` 不设按 IP 限流。签名服务所在目录与 `downloads.json` 的 `base_url` 要一起更新：

```json
"base_url": "/download/v0.5.2/"
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

## 更新源（已切到 OSS 网关）

`latest.json` 已放进私有 Bucket 的根目录，由下载网关提供。备案期间地址仍用 HTTP + IP：

```text
http://8.152.100.169/latest.json
```

清单里的 `platforms[*].url` 指向 `http://8.152.100.169/download/v<版本>/<文件>`（`OSS_PUBLIC_BASE`），由网关 302 到私有 OSS；`signature` 保持 Tauri 生成的值不变。2026-11-08 已把 `features/updates.rs` 的 `DEFAULT_ENDPOINT`、`release.yml` 的 `XJTU_UPDATE_ENDPOINT`、GitHub 变量 `XJTU_UPDATE_ENDPOINT` 与 `OSS_PUBLIC_BASE` 都改成上面的 IP 地址；0.5.x 客户端内置的旧路径 `/xjtu-launcher/latest.json` 由 nginx 精确匹配转发到网关，旧用户也能收到新版本清单。

Tauri 更新器使用 reqwest，默认跟随重定向；**尚未用真实旧版客户端走通「检查更新 → 下载 → 校验 → 安装 → 重启」**。备案完成、域名与证书就绪后，按 [P0.md](P0.md) 的退出条件换成 `https://<域名>/latest.json`，恢复三处 HTTPS-only 校验，并把 `OSS_PUBLIC_BASE` 改回域名地址。

## 验收状态

- 官网页面：已实现，本机用 Chrome 在 1440 px 和 390 px 宽度下检查过布局，没有横向溢出；第三轮改动在 1280×720 / 1280×800 / 1440×900 与 500×900 下截图核对，首屏仍能同时看到标签、标题、轮播与下载卡。第四轮改动（标签配色、三栏方向、点赞、关于、Q&A、下载量）在 1440 px 与真实 390 px 视口（CDP `Emulation.setDeviceMetricsOverride`）截图核对，`body.scrollWidth` 等于视口宽度。第五轮改动（标签去圆点加粗、右上角下载量、玻璃下载卡、Q&amp;A 侧边目录）在 1440 px、1280×800 与 390 px 下核对：首屏下载卡底边 741 px < 800，无横向溢出；Q&amp;A 目录默认高亮「常见问题」，点击问题会展开并定位到 `scroll-margin-top`，侧边目录在 390 px 下收成「目录」面板。**尚未经过用户验收。**
- 点赞与下载量接口：代码、systemd 服务与 nginx 路由已部署，`xjtu-site-api` 与 `xjtu-oss-gateway` 均处于 active；`/site/api/stats` 与 `/api/stats`（带 `Host` 头）都返回真实数据。2026-11-08 验证：经 `127.0.0.1:8787` 和公网 `/download/` 各发一次不带 `Range` 的安装包 `GET`，`gateway-downloads.json` 与 `/site/api/stats` 的总数同步 +1（46 → 48）；`HEAD`、`Range`、`latest.json`、`.sig` 均不计数；验证后清掉 `gateway-downloads.json`，线上起算值为 GitHub 的 46。点赞：新浏览器配置下点击方向按钮返回「已支持」且计数与 localStorage 一致，服务端按加盐 IP 摘要去重；测试票已清，线上为 0/0/0。**尚未有真实访客数据，也未经过用户验收。**
- 社区广场：`/site/community/` 返回 200，首页与 Q&amp;A 导航已加「社区广场」；桌面单条约 327×600 px，390 px 下约 110×490 px，`body.scrollWidth` 等于视口宽度，390 px 导航保留「社区广场 / 下载 / GitHub」；`band-in`、`bar-float` 动画与 `prefers-reduced-motion` 关闭规则已用计算样式确认。**页面只有占位说明，尚未对接应用内社区内容，最新视觉尚未经过用户验收。**
- Q&A 页面：`/site/doc/qa/` 与 `qa.js` 返回 200；左侧目录在桌面与 390 px 下均已截图核对，点击问题链接会展开并定位（`top` 约 76 px，与 `scroll-margin-top` 一致）。内容按 v0.5.2 编写，**未逐条与用户确认**。
- 域名解析：**当前不可用**。备案期间阿里云不允许解析，权威 NS 无 A 记录，公共 DNS 均为空；带 `Host: xjtu-launcher.cn` 访问 IP 仍被 `Server: Beaver` 以未备案拦截（403）。备案通过后再恢复解析并申请证书。
- IP 预览：`http://8.152.100.169/site/` 已加 `X-Robots-Tag: noindex, nofollow`，备案期间不被搜索引擎收录；站点、Q&A、点赞接口、OSS 下载均已在该地址验证可用。
- 部署：2026-11-08 已完成，第五轮改动已重新上传并核对线上页面。`http://8.152.100.169/site/` 返回 200，`styles.css` / `app.js` / `downloads.json` / `doc/qa/` / `doc/qa/qa.js` 均 200；带 `Host: xjtu-launcher.cn` 访问也返回 200。上线前在服务器上删掉了 `likes.json` 里的 1/1/1 测试票并重启服务，线上点赞计数为 0/0/0。
- 2026-11-10 部署：轮播改为 2.5 秒一张（先试 1.5 秒，用户觉得快），仍由进度条动画（`dotfill`）驱动切换，`prefers-reduced-motion` 下改用定时器兜底。CSS / JS 引用统一加 `?v=<日期>` 版本参数（本次 `v=20261110b`），**每次改完样式或脚本都要同步改版本号**，否则访客可能沿用缓存。版本占位从写死的 `v0.5.2` 改为 `v—`，`downloads.json` 读取失败时 `showFallback` 也会把占位复位，不再显示旧版本；Q&amp;A 的版本号同步改为 v0.6.0；`app.js` 的接口与清单地址改为按 `document.baseURI` 解析（`/`、`/site/`、预览页共用同一份脚本）。线上核对了 2.5 秒节奏（2521 / 2522 ms）、版本 `v0.6.0`、点赞 3/2/2 与点击开关（首页与 `/site/preview/` 都试过投票并取消）。**首页与 Q&amp;A 的 HTML/JS 没有 `Cache-Control`，仍是启发式缓存；建议在 nginx 的 `/site/` 与域名站点的 `location /` 加 `Cache-Control: no-cache`，待用户确认后再改服务器配置。** 本机若只用 `python3 -m http.server` 看预览，站点没有 `/api/`，点赞会一直显示 `—`、点击提示「暂时不可用」——这是本地缺接口，不是线上故障；本次本机预览已换成「静态目录 + `/api/` 反代到线上接口」的临时脚本（放在 job 临时目录，不入库）。
- 2026-11-10 轮播换图（用户提供 `docs/promo/` 截图，用户随后要求补上「样式 DIY」与「作业交给 Agent」两幕）：五张依次为 `home-clean.jpg`（主页）、`home-style.jpg`（主页壁纸 + 霞鹜文楷）、`campus-tools.jpg`（畅行校园待办）、`community.jpg`（社区广场）、`mcp-agent.jpg`（Agent 会话说作业交给它）。来源都是真实验收包截图，用 `sips` 压到 1280 宽、q82–85；`og:image` 与三处页面引用同步改成 `home-clean.jpg`。`mcp-agent.jpg` 是把 `docs/promo/mcp-codex.png` 的会话**裁到「文件：第二章作业答案.md」为止**、去掉气泡与「用时」之间的空白段后重排成 1280×869：窗口只显示图片上部约 70%、底部还有 64 px 白渐变，所以正文必须落在前 60% 左右才不会被吃掉。裁掉的后续几行是 11-09 修复前的「提交结果：失败」；修复后已用真实账号经 MCP 真实提交成功（见 `docs/promo/README.md`），所以文案写「再通过 MCP 提交」是当前版本的能力，不是这张图证明的。原 MCP 模式卡（端口、地址、17 个工具）不再单独占一屏，卡片信息改成文案里的「本机 MCP 模式开放 17 个校园工具」；要把它加回来就是第六张。社区轮播文案注明「板块内容还在开发中」。`home-style.jpg` 的壁纸是用户自备图，公开宣传的授权由用户确认。旧图 `home.jpg`、`campus.jpg`、`mcp.jpg`、`mcp-tools.jpg`、`mcp-card.jpg` 已无引用，先保留以防缓存中的旧页面 404。
- 2026-11-10 首页并入 `/preview/` 改版稿（用户指定）：`preview/preview.css` 的轮播指针倾斜 / 高光 / 强调色阴影、下载卡边缘光与推荐卡旋转描边，以及 `preview/direction.css` 的整行交替方向区（大号描边编号、`.path-body` 包裹、偶数行图在左）都搬进 `styles.css`；指针逻辑从 `preview/preview.js` 移到新的 `polish.js`（触屏与 `prefers-reduced-motion` 下不加载效果）。首页方向区标记改成和预览页一致，说明文字从「每张卡片下面有一个『支持』」改成「每个方向旁边」。资源版本号统一 `v=20261110c`。**`preview/` 目录现在和首页重复**（同一份样式与标记），以后再改样式要改两处，建议确认后删掉该目录和 `/site/preview/`。线上核对：1440 宽下方向区为交替整行、轮播窗口有倾斜矩阵与 `--accent` 阴影、推荐卡 `ring-turn` 动画在跑；390 宽下方向区单列堆叠，`body.scrollWidth` = 390 无横向溢出。
- 2026-11-10 轮播交互（用户提出）：鼠标移到轮播区（`.stage`）时暂停自动播放并让当前截图放大 10%，移出恢复。暂停复用 `is-paused`（进度条 `dotfill` 一起停），触摸设备不触发（`pointerType` 判断 + `@media (hover: hover)` 限制放大）。放大是居中的，所以 `.carousel-foot` 的间距从 18 px 提到 30 px（短屏 22 / 18，窄屏 20），1440×900 下放大后窗口顶 306 px 仍在简介文字（298 px）之下、底 701 px 仍在说明文字（714 px）之上；1280×800 首屏下载卡底边 751 px < 800；390 宽 `body.scrollWidth` = 390。资源版本 `v=20261110g`。
- 2026-11-10 上线 v0.6.6：`docs/releases/v0.6.6.md` 与 `website/downloads.json`（版本 0.6.6、日期 2026-11-10、`base_url` `/download/v0.6.6/`、三个包的真实大小与 SHA256）已随发版提交 `1477eba` 入库；站点重新打包上传（先传到 `site.new` 再整目录切换，旧目录留作 `site.old`），资源版本 `v=20261110h`。线上核对：首屏版本胶囊 `v0.6.6`、版本行「当前版本 0.6.6 · 2026-11-10」、三张卡片指向 `/download/v0.6.6/` 且大小 24.0 / 24.9 / 21.5 MB、三条 SHA256 明细；轮播五张、`.carousel-foot` 间距 30 px；Q&amp;A 的版本与日期由清单填入 `v0.6.6` / `2026-11-10`；`/download/v0.6.6/XJTU.Launcher_0.6.6_x64-setup.exe` 带 `Range` 请求返回 206 与真正的 PE 头，网关计数从 17 增到 18。整包完整下载仍未用浏览器走一遍。
- 2026-11-10 版本清单自动化（用户选定「流水线生成清单」）：新增 `tools/site-manifest.mjs` 与 `.github/workflows/site-manifest.yml`，`tools/upload-oss.mjs` 发版时顺带上传 `downloads.json`；网关与 nginx 的读法见上文「更新下载信息」。**本次上线的 0.6.6 走的仍是站内静态副本**——桶里还没有 `downloads.json`（网关实测 404，nginx 按设计退回静态文件），因为改动还没进 `main`，发版工作流跑的时候仓库里还没有这段代码。合并进 `main` 后，跑一次 `Site manifest`（输入 `0.6.6`）就能把 0.6.6 的清单补齐，之后每个版本自动更新；在此之前官网版本信息仍以 `website/downloads.json` 为准。
- ICP 备案：**未完成**，域名被阿里云拦截（403 `Non-compliance ICP Filing`），这是域名上线的唯一阻塞项。
- HTTPS：**未完成**，443 未监听；备案前无法完成 HTTP-01 验证。
- OSS：Bucket `xjtu-launcher` 私有（匿名访问 403），RAM 用户与最小权限策略已建；v0.5.2 的 9 个资产与 `latest.json` 已真实上传。
- 下载网关：服务 `active`，`/healthz` 返回 `ok`；签名 302、跟随跳转、Referer 拦截、限流 429 均在公网 IP 预览链路上实测通过。**未用真实旧版客户端验证更新器跟随 302**。
- 自动上传：GitHub 变量与 Secrets 已配置，`release.yml` 的 `oss` job 会在下一版真实执行；**尚未跑过一次真实的发版工作流**。
- 官网下载：`http://8.152.100.169/site/` 的三张卡片已指向 `/download/v0.5.2/...`，点击后 302 到 OSS；实际浏览器完整下载一个 23 MB 安装包**尚未验证**（只做了 1 字节 Range 跟随）。
- 官网内容按 0.5.2 当前能力编写；0.6 功能发布前不要提前写进页面。
