#!/usr/bin/env python3
"""Short-lived signed URLs for the private OSS download bucket.

The service listens on loopback only. nginx owns the per-IP rate limits and
proxies /download/... here; this process turns an allowed object key into a
302 to OSS with a signature that expires in OSS_URL_TTL seconds.

No third-party packages: OSS V1 URL signing is HMAC-SHA1 over
"GET\\n\\n\\n<expires>\\n/<bucket>/<key>".
"""

import base64
import hashlib
import hmac
import os
import re
import sys
import time
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


class ThreadingHTTPServer(ThreadingMixIn, HTTPServer):
    """Alibaba Cloud Linux 3 ships Python 3.6, which lacks the stdlib class."""

    daemon_threads = True

# Only release objects: v0.5.2/XJTU.Launcher_0.5.2_aarch64.dmg and friends.
KEY_RE = re.compile(r"^v\d+\.\d+\.\d+/[A-Za-z0-9][A-Za-z0-9._+-]*$")
SPECIAL_KEYS = {"latest.json"}


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
        elif path.startswith("/download/"):
            key = path[len("/download/") :]
        else:
            self.respond(404, b"not found\n", "text/plain; charset=utf-8", send_body)
            return

        if not allowed_key(key):
            self.respond(404, b"not found\n", "text/plain; charset=utf-8", send_body)
            return

        if not allowed_referer(self.headers.get("Referer", "")):
            self.respond(403, b"forbidden\n", "text/plain; charset=utf-8", send_body)
            return

        location = signed_url(key)
        self.send_response(302)
        self.send_header("Location", location)
        self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", "0")
        self.end_headers()

    def respond(self, status, body, content_type, send_body):
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        if send_body:
            self.wfile.write(body)

    def log_message(self, fmt, *args):
        # Paths are public object names; never log headers or credentials.
        sys.stderr.write("%s - %s\n" % (self.address_string(), fmt % args))


def main():
    if not ACCESS_KEY_ID or not ACCESS_KEY_SECRET:
        sys.exit("OSS_ACCESS_KEY_ID and OSS_ACCESS_KEY_SECRET are required")

    server = ThreadingHTTPServer((BIND, PORT), Handler)
    sys.stderr.write(
        f"xjtu-oss-gateway listening on {BIND}:{PORT}, bucket={BUCKET}, ttl={URL_TTL}s\n"
    )
    server.serve_forever()


if __name__ == "__main__":
    main()
