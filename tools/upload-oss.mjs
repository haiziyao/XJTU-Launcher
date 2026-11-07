#!/usr/bin/env node
/**
 * Upload a release's assets to the private OSS bucket and rewrite latest.json
 * so every platform URL goes through the download gateway.
 *
 * Requires ossutil on PATH. Credentials live in the file passed as
 * OSS_CONFIG_FILE (an ossutil config), never on the command line.
 *
 * Environment:
 *   VERSION          x.y.z
 *   OSS_BUCKET       e.g. xjtu-launcher
 *   OSS_PUBLIC_BASE  e.g. https://xjtu-launcher.cn/download
 *   OSS_CONFIG_FILE  path to an ossutil config with endpoint + AccessKey
 *   ASSET_DIR        directory holding the release assets (default: assets)
 */

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import path from "node:path";

const version = process.env.VERSION || "";
const bucket = process.env.OSS_BUCKET || "";
const publicBase = (process.env.OSS_PUBLIC_BASE || "https://xjtu-launcher.cn/download")
  .replace(/\/+$/, "");
const assetDir = process.env.ASSET_DIR || "assets";
const configFile = process.env.OSS_CONFIG_FILE || "";

function fail(message) {
  console.error(`upload-oss: ${message}`);
  process.exit(1);
}

if (!/^\d+\.\d+\.\d+$/.test(version)) fail("VERSION must look like 0.5.3");
if (!bucket) fail("OSS_BUCKET is required");
if (!configFile || !existsSync(configFile)) {
  fail("OSS_CONFIG_FILE must point to an ossutil config file");
}

const updaterAssets = {
  "darwin-aarch64": `XJTU.Launcher_${version}_aarch64.app.tar.gz`,
  "darwin-aarch64-app": `XJTU.Launcher_${version}_aarch64.app.tar.gz`,
  "darwin-x86_64": `XJTU.Launcher_${version}_x64.app.tar.gz`,
  "darwin-x86_64-app": `XJTU.Launcher_${version}_x64.app.tar.gz`,
  "windows-x86_64": `XJTU.Launcher_${version}_x64-setup.exe`,
  "windows-x86_64-nsis": `XJTU.Launcher_${version}_x64-setup.exe`,
};

const required = new Set([
  ...Object.values(updaterAssets),
  `XJTU.Launcher_${version}_aarch64.dmg`,
  `XJTU.Launcher_${version}_x64.dmg`,
  "latest.json",
]);

for (const name of required) {
  if (!existsSync(path.join(assetDir, name))) fail(`missing asset ${name}`);
}

function ossutil(args) {
  execFileSync("ossutil", [...args, "-c", configFile], { stdio: "inherit" });
}

for (const name of readdirSync(assetDir).sort()) {
  if (name === "latest.json") continue;
  const local = path.join(assetDir, name);
  const remote = `oss://${bucket}/v${version}/${name}`;
  console.log(`upload ${name} -> ${remote}`);
  ossutil(["cp", "-f", local, remote]);
}

const manifestPath = path.join(assetDir, "latest.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
if (manifest.version !== version) {
  fail(`latest.json declares ${manifest.version}, expected ${version}`);
}

for (const [platform, file] of Object.entries(updaterAssets)) {
  if (!manifest.platforms || !manifest.platforms[platform]) {
    fail(`latest.json is missing platform ${platform}`);
  }
  manifest.platforms[platform].url = `${publicBase}/v${version}/${file}`;
}

writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`upload latest.json -> oss://${bucket}/latest.json`);
ossutil(["cp", "-f", manifestPath, `oss://${bucket}/latest.json`]);

console.log(`uploaded v${version} and latest.json to ${bucket}`);
