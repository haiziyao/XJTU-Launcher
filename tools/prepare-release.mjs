import { mkdir, readFile, writeFile } from 'node:fs/promises';
const endpoint = process.env.XJTU_UPDATE_ENDPOINT;
const pubkey = process.env.XJTU_UPDATE_PUBLIC_KEY;
if (!endpoint || !pubkey)
  throw new Error(
    'Set XJTU_UPDATE_ENDPOINT and XJTU_UPDATE_PUBLIC_KEY before building a signed release.',
  );
const url = new URL(endpoint);
if (url.protocol !== 'https:' || url.username || url.password)
  throw new Error('The update endpoint must use HTTPS without credentials.');
const pkg = JSON.parse(await readFile('package.json', 'utf8'));
const tauri = JSON.parse(await readFile('src-tauri/tauri.conf.json', 'utf8'));
const cargo = await readFile('src-tauri/Cargo.toml', 'utf8');
if (pkg.version !== tauri.version || !cargo.includes(`version = "${pkg.version}"`))
  throw new Error('Synchronize app versions before release.');
const tag = process.env.GITHUB_REF_NAME;
if (tag?.startsWith('v') && tag !== `v${pkg.version}`)
  throw new Error('The tag must match the app version.');
await mkdir('.release', { recursive: true });
await writeFile(
  '.release/tauri.conf.json',
  JSON.stringify(
    {
      bundle: { createUpdaterArtifacts: true },
      plugins: { updater: { pubkey, endpoints: [endpoint] } },
    },
    null,
    2,
  ),
);
console.log(`Prepared signed update build for ${pkg.version}`);
