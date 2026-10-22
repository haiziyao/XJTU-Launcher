# 安装包签名与公证（2026-10-23）

当前 v0.3.0 的 macOS 安装包**未签名、未公证**，Windows 安装包**未做 Authenticode 签名**：macOS 首次打开会被 Gatekeeper 拦截（需右键打开或 `xattr -d com.apple.quarantine`），Windows 会弹 SmartScreen“未知发布者”。Tauri 更新包自身还有一套 minisign 签名（`TAURI_SIGNING_PRIVATE_KEY`），与系统证书无关，已配置。

自签名证书不能消除上述提示，只能使用公共 CA 的代码签名证书。

## macOS：Developer ID + 公证

1. 加入 Apple Developer Program（个人 99 美元/年）。
2. 在 Certificates 中创建 **Developer ID Application** 证书，下载后导入钥匙串，导出为带密码的 `.p12`。
3. 在 appleid.apple.com 生成 App 专用密码（用于公证）。
4. 仓库 Secrets 增加：
   - `APPLE_CERTIFICATE`：`.p12` 的 Base64（`base64 -i cert.p12 | pbcopy`）
   - `APPLE_CERTIFICATE_PASSWORD`：导出密码
   - `APPLE_SIGNING_IDENTITY`：`Developer ID Application: 姓名 (TEAMID)`
   - `APPLE_ID`、`APPLE_PASSWORD`（App 专用密码）、`APPLE_TEAM_ID`
5. 在 `.github/workflows/release.yml` 的 macOS 矩阵任务里，构建前导入证书，例如：

   ```yaml
   - name: Import signing certificate
     if: matrix.os == 'macos-latest'
     uses: apple-actions/import-codesign-certs@v3
     with:
       p12-file-base64: ${{ secrets.APPLE_CERTIFICATE }}
       p12-password: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
   ```

   `tauri-action` 检测到 `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` 后会自动签名并公证；也可再设置 `APPLE_SIGNING_IDENTITY` 明确指定证书。

## Windows：Authenticode 代码签名

微软从 2023 年起要求 OV/EV 证书使用硬件令牌或云 HSM，因此可选：

1. **Azure Trusted Signing**（推荐，约 10 美元/月）：个人或企业通过身份验证后可用。用 `trusted-signing-cli` 作为 `signCommand`：
   ```json
   "bundle": {
     "windows": {
       "signCommand": {
         "cmd": "trusted-signing-cli",
         "args": ["-e", "https://eus.codesigning.azure.net/", "-a", "<account>", "-c", "<profile>", "-d", "XJTU Launcher", "%1"]
       }
     }
   }
   ```
   在 CI 里登录 Azure（`azure/login`）并安装该 CLI；证书不经过仓库。
2. **SSL.com eSigner / DigiCert KeyLocker 等云签名**：拿到 OV/EV 证书后用厂商 CLI 作为 `signCommand`（如 `smctl sign`）。
3. **传统 PFX**（仅当 CA 仍提供文件式证书）：把 `.pfx` Base64 存入 `WINDOWS_CERTIFICATE`，密码存入 `WINDOWS_CERTIFICATE_PASSWORD`，CI 中导入证书并把指纹写进临时配置的 `bundle.windows.certificateThumbprint`。
4. **SignPath.io**：开源项目可申请免费代码签名服务，走其 GitHub App 流程。

所有方案都需要在 `tools/prepare-release.mjs` 生成的临时配置里设置 `bundle.windows.signCommand`（或 `certificateThumbprint`），仓库中不要保存私钥。

## 验证清单

- macOS：`codesign -dv --verbose=4 XJTU\ Launcher.app` 显示 Developer ID；`spctl -a -vv` 通过；首次安装不再出现“无法验证开发者”。
- Windows：`signtool verify /pa XJTU.Launcher_0.3.0_x64-setup.exe` 通过；SmartScreen 对新证书仍需少量下载量积累信誉，EV/Trusted Signing 即时生效。
- 签名只影响系统信任提示，不影响自动更新；更新包清单 `latest.json` 的 `signature` 字段仍由 Tauri 密钥生成。
