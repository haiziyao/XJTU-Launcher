// Local/deployable data-only plugin catalog. Production deployments should terminate HTTPS.
import http from 'node:http';
import { createHash } from 'node:crypto';
import { readFile, readdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const defaultDirectory = fileURLToPath(new URL('./packages/', import.meta.url));
const directory = path.resolve(process.env.PLUGIN_PACKAGE_DIR || defaultDirectory);
const host = process.env.PLUGIN_HOST || '127.0.0.1';
const port = Number(process.env.PLUGIN_PORT || 17321);
async function inventory() {
  const files = (await readdir(directory)).filter((name) => name.endsWith('.json')).sort();
  const packages = new Map();
  const plugins = [];
  const ids = new Set();
  for (const name of files) {
    const bytes = await readFile(path.join(directory, name));
    const plugin = JSON.parse(bytes);
    if (
      bytes.length > 65536 ||
      plugin.schemaVersion !== 1 ||
      plugin.page?.type !== 'links' ||
      !/^[a-z0-9][a-z0-9.-]{2,79}$/.test(plugin.id) ||
      plugin.id.includes('..') ||
      ids.has(plugin.id)
    ) {
      throw new Error(`Invalid or duplicate package: ${name}`);
    }
    ids.add(plugin.id);
    const sha256 = createHash('sha256').update(bytes).digest('hex');
    const downloadUrl = `packages/${sha256}.json`;
    packages.set(`/${downloadUrl}`, bytes);
    plugins.push({
      id: plugin.id,
      name: plugin.name,
      description: plugin.description,
      version: plugin.version,
      minAppVersion: '0.1.0',
      downloadUrl,
      sha256,
    });
  }
  return { packages, catalog: Buffer.from(JSON.stringify({ schemaVersion: 1, plugins })) };
}
await inventory();
const server = http.createServer(async (req, res) => {
  if (!['GET', 'HEAD'].includes(req.method)) {
    res.writeHead(405, { Allow: 'GET, HEAD' });
    res.end();
    return;
  }
  try {
    const current = await inventory();
    const pathname = new URL(req.url, 'http://localhost').pathname;
    const body = pathname === '/catalog.json' ? current.catalog : current.packages.get(pathname);
    if (!body) {
      res.writeHead(404);
      res.end();
      return;
    }
    res.writeHead(200, {
      'Content-Type': 'application/json; charset=utf-8',
      'Content-Length': body.length,
      'Cache-Control':
        pathname === '/catalog.json' ? 'no-store' : 'public, max-age=31536000, immutable',
    });
    res.end(req.method === 'HEAD' ? undefined : body);
  } catch {
    res.writeHead(500);
    res.end('Plugin catalog is unavailable');
  }
});
server.listen(port, host, () => process.stdout.write(`Plugin server: http://${host}:${port}\n`));
