import { createHash } from 'node:crypto';
import { copyFile, mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const target = process.argv[2];
const extensions = {
  'aarch64-apple-darwin': ['.dmg'],
  'x86_64-apple-darwin': ['.dmg'],
  'x86_64-pc-windows-msvc': ['.exe', '.msi'],
  'x86_64-unknown-linux-gnu': ['.deb', '.AppImage'],
}[target];
if (!extensions) throw new Error(`Unsupported release target: ${target}`);
const root = path.join('src-tauri', 'target', target, 'release', 'bundle');
const output = '.release/assets';
await mkdir(output, { recursive: true });
const checksums = [];
for (const extension of extensions) {
  const files = (await readdir(root, { recursive: true })).filter((file) =>
    file.endsWith(extension),
  );
  if (files.length !== 1)
    throw new Error(`Expected one ${extension} installer, found ${files.length}`);
  const source = path.join(root, files[0]);
  const name = path.basename(source);
  await copyFile(source, path.join(output, name));
  checksums.push(
    `${createHash('sha256')
      .update(await readFile(source))
      .digest('hex')}  ${name}`,
  );
}
await writeFile(path.join(output, `SHA256SUMS-${target}.txt`), checksums.join('\n') + '\n');
console.log(`Collected ${extensions.length} installers for ${target}`);
