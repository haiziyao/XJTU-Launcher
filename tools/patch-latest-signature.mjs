#!/usr/bin/env node
/**
 * Replace the Windows updater signatures in a release's latest.json.
 *
 * Signing the NSIS installer after Tauri built it changes its bytes, so the
 * minisign signature Tauri generated for the unsigned file no longer matches.
 * Re-sign the installer with `tauri signer sign` first, then point both
 * Windows entries at the new signature with this script.
 *
 * Usage: node tools/patch-latest-signature.mjs <latest.json> <installer.exe.sig>
 */

import { readFileSync, writeFileSync } from 'node:fs';

const [manifestPath, signaturePath] = process.argv.slice(2);
if (!manifestPath || !signaturePath) {
  console.error('usage: patch-latest-signature.mjs <latest.json> <installer.exe.sig>');
  process.exit(1);
}

const signature = readFileSync(signaturePath, 'utf8').trim();
if (!signature) {
  console.error(`empty signature file: ${signaturePath}`);
  process.exit(1);
}

const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
for (const platform of ['windows-x86_64', 'windows-x86_64-nsis']) {
  if (!manifest.platforms?.[platform]) {
    console.error(`latest.json has no platform ${platform}`);
    process.exit(1);
  }
  manifest.platforms[platform].signature = signature;
}

writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`patched ${manifestPath} with the signed Windows installer signature`);
