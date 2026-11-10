#!/usr/bin/env python3
"""Short-lived signed URLs for the private OSS download bucket.

The service listens on loopback only. nginx owns the per-IP rate limits and
proxies /download/... here; this process turns an allowed object key into a
302 to OSS with a signature that expires in OSS_URL_TTL seconds.

No third-party packages: OSS V1 URL signing is HMAC-SHA1 over
"GET\\n\\n\\n<expires>\\n/<bucket>/<key>".
"""

import base64
import datetime
import hashlib
import hmac
import json
import os
import re
import sys
import threading
import time
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, HTTPServer
from socketserver import ThreadingMixIn
from urllib.parse import quote

BUCKET = os.environ.get("OSS_BUCKET", "xjtu-launcher")
REGION = os.environ.get("OSS_REGION", "oss-cn-beijing")
ACCESS_KEY_ID = os.environ.get("OSS_ACCESS_KEY_ID", "")
ACCESS_KEY_SECRET = os.environ.get("OSS_ACCESS_KEY_SECRET", "")
PUBLIC_HOST = os.environ.get("OSS_PUBLIC_HOST") or f"{BUCKET}.{REGION}.aliyuncs.com"
URL_TTL = int(os.environ.get("OSS_URL_TTL", "60"))
BIND = os.environ.get("GATEWAY_BIND", "127.0.0.1")
PORT = int(os.environ.get("GATEWAY_PORT", "8787"))
SITE_ORIGIN = os.environ.get("SITE_ORIGIN", "https://xjtu-launcher.cn")
# Optional download counter shared with site-api. Empty disables counting.
DOWNLOAD_STATS_FILE = os.environ.get("DOWNLOAD_STATS_FILE", "")
DOWNLOAD_LOG_FILE = os.environ.get("DOWNLOAD_LOG_FILE", "")
if not DOWNLOAD_LOG_FILE and DOWNLOAD_STATS_FILE:
    DOWNLOAD_LOG_FILE = os.path.join(
        os.path.dirname(DOWNLOAD_STATS_FILE), "downloads.log"
    )
COUNT_EXTS = (".dmg", ".exe", ".msi", ".appimage", ".deb", ".tar.gz")
# Per-browser quota. Campus and office NATs put many people behind one IP, so
# the fair limit is a first-party cookie; nginx keeps a much looser per-IP cap.
DOWNLOAD_COOKIE = "xl_dl"
DOWNLOAD_QUOTA = int(os.environ.get("DOWNLOAD_QUOTA", "20"))
DOWNLOAD_QUOTA_WINDOW = int(os.environ.get("DOWNLOAD_QUOTA_WINDOW", "600"))
COOKIE_MAX_AGE = 365 * 24 * 3600
COOKIE_RE = re.compile(r"^[0-9a-f]{32}$")
LOG_MAX_BYTES = 5 * 1024 * 1024
LOG_BACKUPS = 5
count_lock = threading.Lock()
log_lock = threading.Lock()
quota_lock = threading.Lock()
quota_hits = {}


def is_installer(key):
    return not key.endswith(".sig") and key.lower().endswith(COUNT_EXTS)


def cookie_key(value):
    return hashlib.sha256(value.encode("utf-8")).hexdigest()[:16]


def allow_download(value):
    """Sliding-window quota per browser cookie; the cookie is never logged."""
    key = cookie_key(value)
    now = time.time()
    with quota_lock:
        hits = [stamp for stamp in quota_hits.get(key, []) if now - stamp < DOWNLOAD_QUOTA_WINDOW]
        if len(hits) >= DOWNLOAD_QUOTA:
            quota_hits[key] = hits
            return False
        hits.append(now)
        quota_hits[key] = hits
        if len(quota_hits) > 5000:
            for other, stamps in list(quota_hits.items()):
                if not stamps or now - stamps[-1] >= DOWNLOAD_QUOTA_WINDOW:
                    quota_hits.pop(other, None)
        return True


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


def log_event(event, **fields):
    """Append one JSON line to the download audit log; never breaks a download."""
    if not DOWNLOAD_LOG_FILE:
        return
    fields["t"] = datetime.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")
    fields["event"] = event
    line = json.dumps(fields, ensure_ascii=False, sort_keys=True) + "\n"
    with log_lock:
        try:
            os.makedirs(os.path.dirname(DOWNLOAD_LOG_FILE), exist_ok=True)
            rotate_log(DOWNLOAD_LOG_FILE)
            with open(DOWNLOAD_LOG_FILE, "a", encoding="utf-8") as handle:
                handle.write(line)
                handle.flush()
        except (IOError, OSError) as error:
            sys.stderr.write("cannot write download log: %s\n" % error)


class ThreadingHTTPServer(ThreadingMixIn, HTTPServer):
    """Alibaba Cloud Linux 3 ships Python 3.6, which lacks the stdlib class."""

    daemon_threads = True

# Only release objects: v0.5.2/XJTU.Launcher_0.5.2_aarch64.dmg and friends.
KEY_RE = re.compile(r"^v\d+\.\d+\.\d+/[A-Za-z0-9][A-Za-z0-9._+-]*$")
# latest.json is the app's update manifest; downloads.json is what the website
# reads for the version pill, card sizes and SHA256 list.
SPECIAL_KEYS = {"latest.json", "downloads.json"}
MANIFEST_MAX_BYTES = 256 * 1024


def allowed_key(key):
    return key in SPECIAL_KEYS or bool(KEY_RE.match(key))


def signed_url(key):
    expires = int(time.time()) + URL_TTL
    resource = f"/{BUCKET}/{key}"
    string_to_sign = f"GET\n\n\n{expires}\n{resource}"
    digest = hmac.new(
        ACCESS_KEY_SECRET.encode("utf-8"),
        string_to_sign.encode("utf-8"),
        hashlib.sha1,
    ).digest()
    signature = quote(base64.b64encode(digest).decode("ascii"), safe="")
    return (
        f"https://{PUBLIC_HOST}/{quote(key, safe='/')}"
        f"?OSSAccessKeyId={quote(ACCESS_KEY_ID, safe='')}"
        f"&Expires={expires}&Signature={signature}"
    )


def count_download(key):
    """Increment the persistent download counter for a release object.

    Counts download starts (a fresh GET without a Range header), not resumed or
    ranged requests. Signature files are ignored. Failures never break a
    download; the counter is best effort and site-api treats a missing file as
    zero.
    """
    if not DOWNLOAD_STATS_FILE or not is_installer(key):
        return
    try:
        with count_lock:
            counts = {"total": 0, "files": {}}
            try:
                with open(DOWNLOAD_STATS_FILE, encoding="utf-8") as handle:
                    saved = json.load(handle)
            except (IOError, ValueError):
                saved = None
            if isinstance(saved, dict):
                total = saved.get("total")
                if isinstance(total, int) and total >= 0:
                    counts["total"] = total
                files = saved.get("files")
                if isinstance(files, dict):
                    counts["files"] = {
                        str(name): value
                        for name, value in files.items()
                        if isinstance(value, int) and value >= 0
                    }
            counts["total"] += 1
            counts["files"][key] = counts["files"].get(key, 0) + 1
            counts["updated_at"] = int(time.time())

            directory = os.path.dirname(DOWNLOAD_STATS_FILE)
            if directory:
                os.makedirs(directory, exist_ok=True)
            tmp = DOWNLOAD_STATS_FILE + ".tmp"
            with open(tmp, "w", encoding="utf-8") as handle:
                json.dump(counts, handle, ensure_ascii=False, indent=1)
                handle.flush()
                os.fsync(handle.fileno())
            os.replace(tmp, DOWNLOAD_STATS_FILE)
    except (IOError, OSError) as error:
        sys.stderr.write("cannot record download: %s\n" % error)
        log_event("download_count_failed", key=key, error=str(error))
        return
    log_event("download", key=key, total=counts["total"], file_total=counts["files"][key])


def allowed_referer(referer):
    """Allow direct requests (updater, curl) and the official site only.

    A missing Referer is allowed because the desktop updater does not send one.
    SITE_ORIGIN accepts several comma-separated origins so the IP preview keeps
    working before the domain is available; the preview entry is removed once
    the domain is live. This is a mild hotlink deterrent, not an authorization
    check.
    """
    if not referer:
        return True
    origins = [o.strip().rstrip("/") + "/" for o in SITE_ORIGIN.split(",") if o.strip()]
    return any(referer.startswith(origin) for origin in origins)


class Handler(BaseHTTPRequestHandler):
    server_version = "xjtu-oss-gateway"
    sys_version = ""

    def do_GET(self):
        self.handle_request(send_body=True)

    def do_HEAD(self):
        self.handle_request(send_body=False)

    def handle_request(self, send_body):
        path = self.path.split("?", 1)[0]

        if path == "/healthz":
            self.respond(200, b"ok\n", "text/plain; charset=utf-8", send_body)
            return

        if path == "/latest.json":
            key = "latest.json"
        elif path == "/downloads.json":
            key = "downloads.json"
        elif path.startswith("/download/"):
            key = path[len("/download/") :]
        else:
            self.respond(404, b"not found\n", "text/plain; charset=utf-8", send_body)
            return

        if not allowed_key(key):
            log_event("bad_key", key=key[:120])
            self.respond(404, b"not found\n", "text/plain; charset=utf-8", send_body)
            return

        if not allowed_referer(self.headers.get("Referer", "")):
            # The Referer value itself is not logged: it is attacker-controlled
            # and can be long or contain personal paths.
            log_event("referer_rejected", key=key)
            self.respond(403, b"forbidden\n", "text/plain; charset=utf-8", send_body)
            return

        # The site manifest is a few hundred bytes and every page view needs it,
        # so serve the bytes instead of a 302: nginx can fall back to the static
        # copy when the object is absent, and a broken object never reaches the
        # browser. Not counted as a download.
        if key == "downloads.json":
            self.serve_json(key, send_body)
            return

        # A fresh GET is one download start; ignore HEAD, manifests and Range
        # requests (resume / partial reads).
        set_cookie = None
        if send_body and key != "latest.json" and not self.headers.get("Range") and is_installer(key):
            value = self.cookie_value()
            if not value:
                value = os.urandom(16).hex()
                set_cookie = "%s=%s; Path=/; Max-Age=%d; HttpOnly; SameSite=Lax" % (
                    DOWNLOAD_COOKIE,
                    value,
                    COOKIE_MAX_AGE,
                )
            if not allow_download(value):
                log_event("download_quota_exceeded", key=key)
                self.respond(
                    429,
                    b"too many downloads, try again later\n",
                    "text/plain; charset=utf-8",
                    send_body,
                    set_cookie=set_cookie,
                    retry_after=DOWNLOAD_QUOTA_WINDOW,
                )
                return
            count_download(key)
        elif send_body and key == "latest.json":
            log_event("manifest")

        location = signed_url(key)
        self.send_response(302)
        self.send_header("Location", location)
        self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", "0")
        if set_cookie:
            self.send_header("Set-Cookie", set_cookie)
        self.end_headers()

    def serve_json(self, key, send_body):
        """Fetch a manifest object and hand the bytes to the caller.

        A missing object stays a 404 so nginx can serve the static fallback; a
        bad signature, unreachable OSS or malformed JSON becomes 502 and the
        stale copy is not shown as if it were current.
        """
        try:
            with urllib.request.urlopen(signed_url(key), timeout=10) as response:
                body = response.read(MANIFEST_MAX_BYTES + 1)
        except urllib.error.HTTPError as error:
            if error.code == 404:
                self.respond(404, b"not found\n", "text/plain; charset=utf-8", send_body)
            else:
                self.respond(502, b"manifest unavailable\n", "text/plain; charset=utf-8", send_body)
            return
        except Exception as error:  # socket timeouts, DNS, TLS, OSS 5xx
            log_event("manifest_failed", key=key, error=str(error)[:200])
            self.respond(502, b"manifest unavailable\n", "text/plain; charset=utf-8", send_body)
            return

        if len(body) > MANIFEST_MAX_BYTES:
            log_event("manifest_failed", key=key, error="too large")
            self.respond(502, b"manifest unavailable\n", "text/plain; charset=utf-8", send_body)
            return
        try:
            json.loads(body.decode("utf-8"))
        except ValueError:
            log_event("manifest_failed", key=key, error="invalid json")
            self.respond(502, b"manifest unavailable\n", "text/plain; charset=utf-8", send_body)
            return

        # no-cache: a published release must show up on the next page load.
        self.respond(
            200,
            body,
            "application/json; charset=utf-8",
            send_body,
            cache_control="no-cache",
        )

    def respond(
        self,
        status,
        body,
        content_type,
        send_body,
        set_cookie=None,
        retry_after=None,
        cache_control=None,
    ):
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        if cache_control:
            self.send_header("Cache-Control", cache_control)
        if set_cookie:
            self.send_header("Set-Cookie", set_cookie)
        if retry_after:
            self.send_header("Retry-After", str(retry_after))
        self.end_headers()
        if send_body:
            self.wfile.write(body)

    def cookie_value(self):
        header = self.headers.get("Cookie", "")
        for part in header.split(";"):
            name, _, value = part.strip().partition("=")
            if name == DOWNLOAD_COOKIE and COOKIE_RE.match(value):
                return value
        return ""

    def log_message(self, fmt, *args):
        # Paths are public object names; never log headers or credentials.
        sys.stderr.write("%s - %s\n" % (self.address_string(), fmt % args))


def main():
    if not ACCESS_KEY_ID or not ACCESS_KEY_SECRET:
        sys.exit("OSS_ACCESS_KEY_ID and OSS_ACCESS_KEY_SECRET are required")

    log_event(
        "start",
        bucket=BUCKET,
        ttl=URL_TTL,
        counter=bool(DOWNLOAD_STATS_FILE),
        quota=DOWNLOAD_QUOTA,
        quota_window=DOWNLOAD_QUOTA_WINDOW,
    )
    server = ThreadingHTTPServer((BIND, PORT), Handler)
    sys.stderr.write(
        f"xjtu-oss-gateway listening on {BIND}:{PORT}, bucket={BUCKET}, ttl={URL_TTL}s\n"
    )
    server.serve_forever()


if __name__ == "__main__":
    main()
