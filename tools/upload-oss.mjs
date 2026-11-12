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
 *   NOTES_FILE       release notes for latest.json.notes
 *                    (default: docs/releases/v<version>.md)
 *   MANIFEST_ONLY    set to 1 to rewrite only latest.json from ASSET_DIR,
 *                    keeping its platform URLs as they are
 */

import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';

import { buildSiteManifest } from './site-manifest.mjs';

const version = process.env.VERSION || '';
const bucket = process.env.OSS_BUCKET || '';
const publicBase = (process.env.OSS_PUBLIC_BASE || 'https://xjtu-launcher.cn/download').replace(
  /\/+$/,
  '',
);
const assetDir = process.env.ASSET_DIR || 'assets';
const configFile = process.env.OSS_CONFIG_FILE || '';
const notesFile = process.env.NOTES_FILE || path.join('docs', 'releases', `v${version}.md`);
const manifestOnly = process.env.MANIFEST_ONLY === '1';

function fail(message) {
  console.error(`upload-oss: ${message}`);
  process.exit(1);
}

if (!/^\d+\.\d+\.\d+$/.test(version)) fail('VERSION must look like 0.5.3');
if (!bucket) fail('OSS_BUCKET is required');
if (!configFile || !existsSync(configFile)) {
  fail('OSS_CONFIG_FILE must point to an ossutil config file');
}

const updaterAssets = {
  'darwin-aarch64': `XJTU.Launcher_${version}_aarch64.app.tar.gz`,
  'darwin-aarch64-app': `XJTU.Launcher_${version}_aarch64.app.tar.gz`,
  'darwin-x86_64': `XJTU.Launcher_${version}_x64.app.tar.gz`,
  'darwin-x86_64-app': `XJTU.Launcher_${version}_x64.app.tar.gz`,
  'windows-x86_64': `XJTU.Launcher_${version}_x64-setup.exe`,
  'windows-x86_64-nsis': `XJTU.Launcher_${version}_x64-setup.exe`,
};

const required = new Set([
  ...Object.values(updaterAssets),
  `XJTU.Launcher_${version}_aarch64.dmg`,
  `XJTU.Launcher_${version}_x64.dmg`,
  'latest.json',
]);

for (const name of required) {
  if (manifestOnly || existsSync(path.join(assetDir, name))) continue;
  fail(`missing asset ${name}`);
}

if (!existsSync(notesFile)) {
  fail(`missing ${notesFile}; every release needs a version record`);
}
const notes = readFileSync(notesFile, 'utf8');
if (notes.includes('YYYY-MM-DD')) fail(`${notesFile} still contains template placeholders`);

function ossutil(args) {
  execFileSync('ossutil', [...args, '-c', configFile], { stdio: 'inherit' });
}

if (!manifestOnly) {
  for (const name of readdirSync(assetDir).sort()) {
    if (name === 'latest.json' || name === 'downloads.json') continue;
    const local = path.join(assetDir, name);
    const remote = `oss://${bucket}/v${version}/${name}`;
    console.log(`upload ${name} -> ${remote}`);
    ossutil(['cp', '-f', local, remote]);
  }
}

const manifestPath = path.join(assetDir, 'latest.json');
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
if (manifest.version !== version) {
  fail(`latest.json declares ${manifest.version}, expected ${version}`);
}

for (const [platform, file] of Object.entries(updaterAssets)) {
  if (!manifest.platforms || !manifest.platforms[platform]) {
    fail(`latest.json is missing platform ${platform}`);
  }
  if (!manifestOnly) manifest.platforms[platform].url = `${publicBase}/v${version}/${file}`;
}

// The app shows this text in its update prompt, so it has to be the same
// version record the GitHub release body uses.
manifest.notes = notes;

writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`upload latest.json -> oss://${bucket}/latest.json`);
ossutil(['cp', '-f', manifestPath, `oss://${bucket}/latest.json`]);

if (manifestOnly) {
  console.log(`repaired latest.json for v${version}; assets left untouched`);
  process.exit(0);
}

// The site reads downloads.json from the bucket root; generating it here keeps
// the homepage version pill, card sizes, SHA256 list and Q&A version in step
// with each release without anyone editing website/downloads.json.
const siteManifest = buildSiteManifest({
  version,
  assetDir,
  pubDate: manifest.pub_date,
});
const sitePath = path.join(assetDir, 'downloads.json');
writeFileSync(sitePath, `${JSON.stringify(siteManifest, null, 2)}\n`);
console.log(`upload downloads.json -> oss://${bucket}/downloads.json`);
ossutil(['cp', '-f', sitePath, `oss://${bucket}/downloads.json`]);

console.log(`uploaded v${version}, latest.json and downloads.json to ${bucket}`);
