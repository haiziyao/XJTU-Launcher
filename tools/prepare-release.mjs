import { mkdir, readFile, writeFile } from 'node:fs/promises';
const endpoint = process.env.XJTU_UPDATE_ENDPOINT;
const pubkey = process.env.XJTU_UPDATE_PUBLIC_KEY;
const signingKey = process.env.TAURI_SIGNING_PRIVATE_KEY;
const signed = Boolean(endpoint || pubkey || signingKey);
if (signed && (!endpoint || !pubkey || !signingKey))
  throw new Error(
    'Signed updates require XJTU_UPDATE_ENDPOINT, XJTU_UPDATE_PUBLIC_KEY and TAURI_SIGNING_PRIVATE_KEY.',
  );
let insecureTransport = false;
if (signed) {
  const url = new URL(endpoint);
  // P0: the mirror is temporarily served over plain HTTP (docs/P0.md).
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password)
    throw new Error('The update endpoint must use HTTP(S) without credentials.');
  if (url.protocol === 'http:') {
    insecureTransport = true;
    console.warn('P0: plain HTTP update mirror; switch to HTTPS once a domain is ready.');
  }
}
const pkg = JSON.parse(await readFile('package.json', 'utf8'));
const tauri = JSON.parse(await readFile('src-tauri/tauri.conf.json', 'utf8'));
const cargo = await readFile('src-tauri/Cargo.toml', 'utf8');
const cargoVersion = cargo.match(/^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1];
const lock = JSON.parse(await readFile('package-lock.json', 'utf8'));
const cargoLock = await readFile('src-tauri/Cargo.lock', 'utf8');
const lockedVersion = cargoLock.match(/name = "xjtu-launcher"\r?\nversion = "([^"]+)"/)?.[1];
if (
  [tauri.version, cargoVersion, lock.version, lock.packages[''].version, lockedVersion].some(
    (v) => v !== pkg.version,
  )
)
  throw new Error('Synchronize app versions before release.');
const tag = process.env.GITHUB_REF_NAME;
if (tag?.startsWith('v') && tag !== `v${pkg.version}`)
  throw new Error('The tag must match the app version.');
await mkdir('.release', { recursive: true });
await writeFile(
  '.release/tauri.conf.json',
  JSON.stringify(
    {
      bundle: { createUpdaterArtifacts: signed },
      plugins: {
        updater: {
          pubkey: signed ? pubkey : '',
          endpoints: signed ? [endpoint] : [],
          // Release builds abort at startup when an endpoint is not HTTPS
          // unless this flag is set; it stays off once the mirror has TLS.
          dangerousInsecureTransportProtocol: signed && insecureTransport,
        },
      },
    },
    null,
    2,
  ),
);
console.log(`Prepared ${signed ? 'signed update' : 'manual download'} build for ${pkg.version}`);
