#!/usr/bin/env python3
"""Likes and download counts for the XJTU Launcher website.

The service listens on loopback only; nginx proxies ``/api/`` here and owns
the per-IP rate limit.

    GET  /stats                    -> {"downloads": {"total": 46}, "likes": {...}}
    POST /like  {"key": "powerful"} -> {"ok": true, "count": 4, "already": false}

Likes are stored in ``STATS_DIR/likes.json``. One vote per direction and per
browser address: the address is salted and hashed before it is written, so the
file never contains a raw IP.

Download totals come from the GitHub releases API and are cached for
``SITE_API_TTL`` seconds. The last real value is mirrored to
``downloads.json``; when GitHub cannot be reached and nothing is cached the
API answers ``"total": null`` and the page hides the counter instead of
showing an invented number.
"""

import hashlib
import json
import os
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from socketserver import ThreadingMixIn
from urllib.request import Request, urlopen

STATS_DIR = os.environ.get("STATS_DIR", "/srv/xjtu-launcher/stats")
BIND = os.environ.get("SITE_API_BIND", "127.0.0.1")
PORT = int(os.environ.get("SITE_API_PORT", "8788"))
REPO = os.environ.get("SITE_API_REPO", "haiziyao/XJTU-Launcher")
TTL = int(os.environ.get("SITE_API_TTL", "600"))
TOKEN = os.environ.get("GITHUB_TOKEN", "")

KEYS = ("powerful", "community", "design")
INSTALLER_EXTS = (".dmg", ".exe", ".msi", ".appimage", ".deb")

LIKES_FILE = os.path.join(STATS_DIR, "likes.json")
DOWNLOAD_FILE = os.path.join(STATS_DIR, "downloads.json")


class ThreadingHTTPServer(ThreadingMixIn, HTTPServer):
    """Alibaba Cloud Linux 3 ships Python 3.6, which lacks the stdlib class."""

    daemon_threads = True


lock = threading.Lock()
state = {"salt": os.urandom(16).hex(), "counts": dict.fromkeys(KEYS, 0), "voters": {}}
downloads = {"total": None, "fetched_at": 0.0}


def save_json(path, payload):
    tmp = path + ".tmp"
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(tmp, "w", encoding="utf-8") as handle:
        json.dump(payload, handle, ensure_ascii=False, indent=1)
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(tmp, path)


def load_state():
    try:
        with open(LIKES_FILE, encoding="utf-8") as handle:
            saved = json.load(handle)
    except (IOError, ValueError):
        return
    salt = saved.get("salt")
    if isinstance(salt, str) and salt:
        state["salt"] = salt
    counts = saved.get("counts") or {}
    for key in KEYS:
        value = counts.get(key)
        if isinstance(value, int) and value >= 0:
            state["counts"][key] = value
    voters = saved.get("voters") or {}
    if isinstance(voters, dict):
        state["voters"] = {
            str(voter): [k for k in keys if k in KEYS]
            for voter, keys in voters.items()
            if isinstance(keys, list)
        }


def load_downloads():
    try:
        with open(DOWNLOAD_FILE, encoding="utf-8") as handle:
            saved = json.load(handle)
    except (IOError, ValueError):
        return
    total = saved.get("total")
    if isinstance(total, int) and total >= 0:
        downloads["total"] = total  # fetched_at stays 0 so the first call refreshes


def fetch_downloads():
    request = Request(
        "https://api.github.com/repos/%s/releases?per_page=100" % REPO,
        headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "xjtu-launcher-site-api",
        },
    )
    if TOKEN:
        request.add_header("Authorization", "Bearer " + TOKEN)

    with urlopen(request, timeout=8) as response:
        releases = json.loads(response.read(4 * 1024 * 1024).decode("utf-8"))

    total = 0
    for release in releases:
        for asset in release.get("assets") or []:
            name = str(asset.get("name", "")).lower()
            if name.endswith(INSTALLER_EXTS):
                total += int(asset.get("download_count") or 0)
    return total


def download_total():
    now = time.time()
    if downloads["total"] is not None and now - downloads["fetched_at"] < TTL:
        return downloads["total"]
    try:
        total = fetch_downloads()
    except Exception as error:  # network, JSON, rate limit: keep the last real value
        sys.stderr.write("github releases fetch failed: %s\n" % error)
        return downloads["total"]
    downloads["total"] = total
    downloads["fetched_at"] = now
    try:
        save_json(DOWNLOAD_FILE, {"total": total, "fetched_at": int(now)})
    except IOError as error:
        sys.stderr.write("cannot cache downloads: %s\n" % error)
    return total


def voter_id(address):
    digest = hashlib.sha256((state["salt"] + "|" + address).encode("utf-8")).hexdigest()
    return digest[:32]


def add_like(key, address):
    with lock:
        voters = state["voters"].setdefault(voter_id(address), [])
        if key in voters:
            return state["counts"][key], True
        voters.append(key)
        state["counts"][key] = state["counts"].get(key, 0) + 1
        save_json(LIKES_FILE, state)
        return state["counts"][key], False


def stats_payload():
    with lock:
        likes = {key: state["counts"].get(key, 0) for key in KEYS}
    return {"downloads": {"total": download_total()}, "likes": likes}


class Handler(BaseHTTPRequestHandler):
    server_version = "xjtu-site-api/1"
    sys_version = ""

    def reply(self, status, payload):
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def client_ip(self):
        forwarded = self.headers.get("X-Real-IP")
        if forwarded:
            return forwarded.strip()
        return self.client_address[0]

    def do_GET(self):
        if self.path in ("/stats", "/stats/"):
            self.reply(200, stats_payload())
        elif self.path in ("/healthz", "/healthz/"):
            self.reply(200, {"ok": True})
        else:
            self.reply(404, {"error": "not found"})

    def do_POST(self):
        if self.path not in ("/like", "/like/"):
            self.reply(404, {"error": "not found"})
            return
        try:
            length = int(self.headers.get("Content-Length") or 0)
        except ValueError:
            length = 0
        if length <= 0 or length > 512:
            self.reply(413, {"error": "bad length"})
            return
        try:
            payload = json.loads(self.rfile.read(length).decode("utf-8"))
        except (ValueError, UnicodeDecodeError):
            self.reply(400, {"error": "bad json"})
            return
        key = payload.get("key") if isinstance(payload, dict) else None
        if key not in KEYS:
            self.reply(400, {"error": "unknown key"})
            return
        try:
            count, already = add_like(key, self.client_ip())
        except IOError as error:
            sys.stderr.write("cannot save likes: %s\n" % error)
            self.reply(500, {"error": "storage"})
            return
        self.reply(200, {"ok": True, "key": key, "count": count, "already": already})

    def log_message(self, fmt, *args):
        pass  # nginx keeps the access log


def main():
    load_state()
    load_downloads()
    server = ThreadingHTTPServer((BIND, PORT), Handler)
    sys.stderr.write("site-api listening on %s:%d\n" % (BIND, PORT))
    server.serve_forever()


if __name__ == "__main__":
    main()
