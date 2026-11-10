#!/usr/bin/env python3
"""Likes and download counts for the XJTU Launcher website.

The service listens on loopback only; nginx proxies ``/api/`` here and owns
the per-IP rate limit.

    GET  /stats                    -> {"downloads": {"total": 46}, "likes": {...}}
    POST /like  {"key": "powerful"} -> {"ok": true, "count": 4, "voted": true}

Likes are stored in ``STATS_DIR/likes.json``. One vote per direction and per
browser address: the address is salted and hashed before it is written, so the
file never contains a raw IP. Posting the same direction again cancels that
vote (``"voted": false``), so the button works as a toggle.

Download totals combine the GitHub releases API (historical downloads, cached
for ``SITE_API_TTL`` seconds and mirrored to ``downloads.json``) with the
gateway counter in ``gateway-downloads.json`` written by the OSS download
gateway. Since installers are served from OSS, GitHub counts no longer grow;
the gateway records each fresh download start. When neither source has a value
the API answers ``"total": null`` and the page hides the counter instead of
showing an invented number.
"""

import datetime
import hashlib
import json
import os
import re
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
SNAPSHOT_INTERVAL = int(os.environ.get("STATS_SNAPSHOT_INTERVAL", "900"))
SNAPSHOT_KEEP = int(os.environ.get("STATS_SNAPSHOT_KEEP", "192"))

KEYS = ("powerful", "community", "design")
INSTALLER_EXTS = (".dmg", ".exe", ".msi", ".appimage", ".deb")
# Votes are tied to a first-party cookie, not the IP: many students share one
# campus NAT address and must still count as different people.
COOKIE_NAME = "xl_voter"
COOKIE_MAX_AGE = 2 * 365 * 24 * 3600
COOKIE_RE = re.compile(r"^[0-9a-f]{32}$")

LIKES_FILE = os.path.join(STATS_DIR, "likes.json")
DOWNLOAD_FILE = os.path.join(STATS_DIR, "downloads.json")
GATEWAY_FILE = os.path.join(STATS_DIR, "gateway-downloads.json")
LIKES_LOG = os.path.join(STATS_DIR, "likes.log")
STATS_LOG = os.path.join(STATS_DIR, "stats.log")
BACKUP_DIR = os.path.join(STATS_DIR, "backups")
LOG_MAX_BYTES = 5 * 1024 * 1024
LOG_BACKUPS = 5
log_lock = threading.Lock()


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


def utc_stamp():
    return datetime.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")


def rotate_log(path):
    try:
        if os.path.getsize(path) < LOG_MAX_BYTES:
            return
    except OSError:
        return
    for index in range(LOG_BACKUPS - 1, 0, -1):
        src = "%s.%d" % (path, index)
        if os.path.exists(src):
            os.replace(src, "%s.%d" % (path, index + 1))
    os.replace(path, path + ".1")


def log_event(event, path=LIKES_LOG, **fields):
    """Append one JSON line to an audit log; logging never breaks a request."""
    fields["t"] = utc_stamp()
    fields["event"] = event
    line = json.dumps(fields, ensure_ascii=False, sort_keys=True) + "\n"
    with log_lock:
        try:
            os.makedirs(os.path.dirname(path), exist_ok=True)
            rotate_log(path)
            with open(path, "a", encoding="utf-8") as handle:
                handle.write(line)
                handle.flush()
        except (IOError, OSError) as error:
            sys.stderr.write("cannot write %s: %s\n" % (path, error))


def snapshot_file(name, stamp, source):
    """Copy one stats file into backups/ and prune old snapshots."""
    target = os.path.join(BACKUP_DIR, "%s-%s.json" % (name, stamp))
    try:
        with open(source, "rb") as handle:
            payload = handle.read()
    except (IOError, OSError):
        return False
    try:
        os.makedirs(BACKUP_DIR, exist_ok=True)
        with open(target, "wb") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
    except (IOError, OSError) as error:
        sys.stderr.write("cannot snapshot %s: %s\n" % (source, error))
        return False

    prefix = name + "-"
    names = sorted(
        entry for entry in os.listdir(BACKUP_DIR)
        if entry.startswith(prefix) and entry.endswith(".json")
    )
    for entry in names[:-SNAPSHOT_KEEP]:
        try:
            os.remove(os.path.join(BACKUP_DIR, entry))
        except OSError:
            pass
    return True


def snapshot_all():
    stamp = datetime.datetime.utcnow().strftime("%Y%m%dT%H%M%SZ")
    likes_ok = snapshot_file("likes", stamp, LIKES_FILE)
    gateway_ok = snapshot_file("downloads", stamp, GATEWAY_FILE)
    with lock:
        counts = {key: state["counts"].get(key, 0) for key in KEYS}
    log_event(
        "snapshot",
        path=STATS_LOG,
        likes=counts,
        likes_saved=likes_ok,
        gateway_saved=gateway_ok,
    )


def snapshot_loop():
    while True:
        time.sleep(SNAPSHOT_INTERVAL)
        try:
            snapshot_all()
        except Exception as error:  # a bad snapshot must not stop the service
            sys.stderr.write("snapshot failed: %s\n" % error)


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
        log_event("github_fetch_failed", path=STATS_LOG, error=str(error))
        return downloads["total"]
    downloads["total"] = total
    downloads["fetched_at"] = now
    log_event("github_total", path=STATS_LOG, total=total)
    try:
        save_json(DOWNLOAD_FILE, {"total": total, "fetched_at": int(now)})
    except IOError as error:
        sys.stderr.write("cannot cache downloads: %s\n" % error)
    return total


def voter_id(raw):
    # The cookie value itself is never stored, only this salted digest.
    digest = hashlib.sha256((state["salt"] + "|cookie|" + raw).encode("utf-8")).hexdigest()
    return digest[:32]


def toggle_like(key, raw):
    """One vote per browser cookie and direction; a second click cancels it."""
    voter = voter_id(raw)
    with lock:
        voters = state["voters"].setdefault(voter, [])
        if key in voters:
            voters.remove(key)
            state["counts"][key] = max(0, state["counts"].get(key, 0) - 1)
            voted = False
            event = "vote_cancel"
        else:
            voters.append(key)
            state["counts"][key] = state["counts"].get(key, 0) + 1
            voted = True
            event = "vote"
        if not voters:
            state["voters"].pop(voter, None)
        save_json(LIKES_FILE, state)
        count = state["counts"][key]
    # Only the salted hash prefix is logged, never the address itself.
    log_event(event, key=key, count=count, voter=voter[:8])
    return count, voted


def gateway_total():
    """Fresh download starts recorded by the download gateway, 0 if unknown."""
    try:
        with open(GATEWAY_FILE, encoding="utf-8") as handle:
            saved = json.load(handle)
    except (IOError, ValueError):
        return 0
    total = saved.get("total") if isinstance(saved, dict) else None
    return total if isinstance(total, int) and total >= 0 else 0


def stats_payload(raw=""):
    with lock:
        likes = {key: state["counts"].get(key, 0) for key in KEYS}
        if raw:
            voter = voter_id(raw)
            voted = {key: key in state["voters"].get(voter, []) for key in KEYS}
        else:
            voted = dict.fromkeys(KEYS, False)
    github = download_total()
    gateway = gateway_total()
    if github is None:
        total = gateway if gateway > 0 else None
    else:
        total = github + gateway
    return {"downloads": {"total": total}, "likes": likes, "voted": voted}


class Handler(BaseHTTPRequestHandler):
    server_version = "xjtu-site-api/1"
    sys_version = ""

    def reply(self, status, payload, set_cookie=None):
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        if set_cookie:
            self.send_header("Set-Cookie", set_cookie)
        self.end_headers()
        self.wfile.write(body)

    def cookie_value(self):
        header = self.headers.get("Cookie", "")
        for part in header.split(";"):
            name, _, value = part.strip().partition("=")
            if name == COOKIE_NAME and COOKIE_RE.match(value):
                return value
        return ""

    def do_GET(self):
        if self.path in ("/stats", "/stats/"):
            self.reply(200, stats_payload(self.cookie_value()))
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
        raw = self.cookie_value()
        set_cookie = None
        if not raw:
            raw = os.urandom(16).hex()
            set_cookie = "%s=%s; Path=/; Max-Age=%d; HttpOnly; SameSite=Lax" % (
                COOKIE_NAME,
                raw,
                COOKIE_MAX_AGE,
            )
        try:
            count, voted = toggle_like(key, raw)
        except IOError as error:
            sys.stderr.write("cannot save likes: %s\n" % error)
            log_event("vote_failed", key=key, error=str(error))
            self.reply(500, {"error": "storage"})
            return
        self.reply(
            200,
            {"ok": True, "key": key, "count": count, "voted": voted},
            set_cookie=set_cookie,
        )

    def log_message(self, fmt, *args):
        pass  # nginx keeps the access log


def main():
    load_state()
    load_downloads()
    server = ThreadingHTTPServer((BIND, PORT), Handler)
    worker = threading.Thread(target=snapshot_loop)
    worker.daemon = True
    worker.start()
    log_event("start", path=STATS_LOG, interval=SNAPSHOT_INTERVAL, keep=SNAPSHOT_KEEP)
    sys.stderr.write("site-api listening on %s:%d\n" % (BIND, PORT))
    server.serve_forever()


if __name__ == "__main__":
    main()
