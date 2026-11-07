# 官网源码

`xjtu-launcher.cn` 的静态页面，没有构建步骤，也没有外部依赖。

```bash
# 本地预览
python3 -m http.server 8899
# 打开 http://127.0.0.1:8899/
```

- 下载信息集中在 `downloads.json`：版本、日期、下载基址、文件名、大小与 SHA256。
- 安装包放 OSS 后，只改 `base_url`，页面代码不用动。
- 部署、nginx、HTTPS 与 OSS 流程见 [dev-docs/WEBSITE.md](../dev-docs/WEBSITE.md)。

改完页面后重新同步：

```bash
rsync -avz --delete website/ root@8.152.100.169:/srv/xjtu-launcher/site/
```
