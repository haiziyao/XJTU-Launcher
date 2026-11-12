#!/usr/bin/env node
/**
 * Build the website's downloads.json from real release assets and upload it to
 * the bucket root, so a release needs no hand edits on the site.
 *
 * Normally called by tools/upload-oss.mjs right after a release is uploaded.
 * The `Site manifest` workflow runs this file directly to seed (or repair) the
 * manifest for a release that already exists.
 *
 * Environment:
 *   VERSION          x.y.z
 *   OSS_BUCKET       e.g. xjtu-launcher
 *   OSS_CONFIG_FILE  path to an ossutil config with endpoint + AccessKey
 *   ASSET_DIR        directory holding the release assets (default: assets)
 *   PUB_DATE         optional release date (YYYY-MM-DD or ISO); defaults to today (UTC)
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const RELEASES_URL = "https://github.com/haiziyao/XJTU-Launcher/releases";

// The three packages the site offers, in card order. Windows shows its own
// label in the browser, so `arch_label` there is only a fallback.
const SITE_FILES = [
  {
    platform: "macOS",
    arch: "arm64",
    arch_label: "Apple 芯片",
    name: (version) => `XJTU.Launcher_${version}_aarch64.dmg`,
  },
  {
    platform: "macOS",
    arch: "x64",
    arch_label: "Intel 芯片",
    name: (version) => `XJTU.Launcher_${version}_x64.dmg`,
  },
  {
    platform: "Windows",
    arch: "x64",
    arch_label: "64 位",
    name: (version) => `XJTU.Launcher_${version}_x64-setup.exe`,
  },
];

export function sha256File(file) {
  const hash = createHash("sha256");
  hash.update(readFileSync(file));
  return hash.digest("hex");
}

function releaseDate(pubDate) {
  const text = String(pubDate || "");
  if (/^\d{4}-\d{2}-\d{2}/.test(text)) return text.slice(0, 10);
  return new Date().toISOString().slice(0, 10);
}

/**
 * The file the site fetches. `base_url` stays site-relative so the same manifest
 * works on the domain and on the IP preview, where nginx forwards /download/ to
 * the gateway.
 */
export function buildSiteManifest({ version, assetDir, pubDate }) {
  return {
    version,
    date: releaseDate(pubDate),
    releases_url: RELEASES_URL,
    base_url: `/download/v${version}/`,
    files: SITE_FILES.map((entry) => {
      const file = path.join(assetDir, entry.name(version));
      const size = statSync(file).size;
      return {
        platform: entry.platform,
        arch: entry.arch,
        arch_label: entry.arch_label,
        name: path.basename(file),
        size,
        sha256: sha256File(file),
      };
    }),
  };
}

function main() {
  const version = process.env.VERSION || "";
  const bucket = process.env.OSS_BUCKET || "";
  const assetDir = process.env.ASSET_DIR || "assets";
  const configFile = process.env.OSS_CONFIG_FILE || "";

  if (!/^\d+\.\d+\.\d+$/.test(version)) {
    console.error("site-manifest: VERSION must look like 0.6.6");
    process.exit(1);
  }
  if (!bucket) {
    console.error("site-manifest: OSS_BUCKET is required");
    process.exit(1);
  }
  if (!configFile || !existsSync(configFile)) {
    console.error("site-manifest: OSS_CONFIG_FILE must point to an ossutil config file");
    process.exit(1);
  }

  const manifest = buildSiteManifest({ version, assetDir, pubDate: process.env.PUB_DATE });
  const local = path.join(assetDir, "downloads.json");
  writeFileSync(local, `${JSON.stringify(manifest, null, 2)}\n`);

  for (const file of manifest.files) {
    console.log(`${file.name} ${file.size} bytes ${file.sha256}`);
  }
  const remote = `oss://${bucket}/downloads.json`;
  console.log(`upload downloads.json -> ${remote}`);
  execFileSync("ossutil", ["cp", "-f", local, remote, "-c", configFile], { stdio: "inherit" });
}

// Runs only when invoked directly; upload-oss.mjs imports the builder instead.
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
