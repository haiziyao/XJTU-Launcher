#!/usr/bin/env python3
"""Tests for the feedback endpoint of site-api.py.

Runs against a temporary STATS_DIR, so it never touches real data:

    python3 tools/site-api/test_feedback.py
"""

import importlib.util
import json
import os
import re
import shutil
import tempfile
import threading
import unittest
from urllib.error import HTTPError
from urllib.request import Request, urlopen

HERE = os.path.dirname(os.path.abspath(__file__))
STATS = tempfile.mkdtemp(prefix="site-api-test-")
os.environ["STATS_DIR"] = STATS
os.environ["SITE_API_BIND"] = "127.0.0.1"
os.environ["SITE_API_PORT"] = "0"

spec = importlib.util.spec_from_file_location("site_api", os.path.join(HERE, "site-api.py"))
api = importlib.util.module_from_spec(spec)
spec.loader.exec_module(api)


def payload(**overrides):
    body = {
        "kind": "idea",
        "title": "希望课表能导出成 ics",
        "body": "现在的课表只能在应用里看，手机日历收不到提醒。",
        "consent": True,
    }
    body.update(overrides)
    return body


def records():
    result = []
    directory = os.path.join(STATS, "feedback")
    if not os.path.isdir(directory):
        return result
    for name in sorted(os.listdir(directory)):
        if not name.endswith(".jsonl"):
            continue
        with open(os.path.join(directory, name), encoding="utf-8") as handle:
            for line in handle:
                if line.strip():
                    result.append(json.loads(line))
    return result


def log_lines():
    path = os.path.join(STATS, "feedback.log")
    if not os.path.exists(path):
        return []
    with open(path, encoding="utf-8") as handle:
        return [json.loads(line) for line in handle if line.strip()]


class SubmitTest(unittest.TestCase):
    def setUp(self):
        shutil.rmtree(os.path.join(STATS, "feedback"), ignore_errors=True)
        if os.path.exists(os.path.join(STATS, "feedback.log")):
            os.remove(os.path.join(STATS, "feedback.log"))
        api.feedback_state["limits"].clear()
        api.feedback_state["recent"].clear()

    def submit(self, voter="a" * 32, **overrides):
        return api.submit_feedback(
            payload(**overrides), voter, "10.0.0.1", "Mozilla/5.0 (Macintosh)", "/feedback/"
        )

    def test_stores_one_json_line(self):
        feedback_id, duplicate = self.submit()
        self.assertFalse(duplicate)
        self.assertRegex(feedback_id, r"^FB-\d{8}-[A-Z2-9]{4}$")
        items = records()
        self.assertEqual(len(items), 1)
        record = items[0]
        self.assertEqual(record["id"], feedback_id)
        self.assertEqual(record["type"], "idea")
        self.assertEqual(record["platform"], "macOS")
        self.assertEqual(record["page"], "/feedback/")
        self.assertEqual(record["steps"], "")
        self.assertRegex(record["ip"], r"^sha256:[0-9a-f]{8}$")
        self.assertNotIn("10.0.0.1", json.dumps(record))

    def test_same_text_twice_returns_first_id(self):
        first, _ = self.submit()
        second, duplicate = self.submit()
        self.assertTrue(duplicate)
        self.assertEqual(first, second)
        self.assertEqual(len(records()), 1)

    def test_steps_only_kept_for_bugs(self):
        self.submit(kind="idea", steps="不应该保存")
        self.assertEqual(records()[-1]["steps"], "")
        shutil.rmtree(os.path.join(STATS, "feedback"), ignore_errors=True)
        api.feedback_state["recent"].clear()
        self.submit(kind="bug", title="评教页面白屏", body="打开评教就白屏，重试也一样。", steps="点评教")
        self.assertEqual(records()[-1]["steps"], "点评教")

    def test_quota_is_per_cookie(self):
        for index in range(api.FEEDBACK_PER_WINDOW):
            self.submit(voter="b" * 32, title="第 %d 条建议" % index, body="描述内容足够长了，这是第 %d 条。" % index)
        with self.assertRaises(api.FeedbackError) as caught:
            self.submit(voter="b" * 32, title="第六条建议", body="描述内容足够长了，这是第六条。")
        self.assertEqual(caught.exception.status, 429)
        # Another browser address is not blocked by the first one's quota.
        self.submit(voter="c" * 32)

    def test_rejects_bad_input(self):
        cases = [
            ({"title": "短"}, "title"),
            ({"title": "   "}, "title"),
            ({"body": "不够长"}, "body"),
            ({"kind": "praise"}, "kind"),
            ({"consent": False}, "consent"),
            ({"website": "http://spam"}, "spam"),
        ]
        for overrides, code in cases:
            with self.assertRaises(api.FeedbackError) as caught:
                self.submit(**overrides)
            self.assertEqual(caught.exception.status, 400)
            self.assertEqual(caught.exception.code, code)
        self.assertEqual(records(), [])

    def test_log_has_no_text_and_flags_secrets(self):
        self.submit(
            title="登录这里好像有问题",
            body="Cookie 是 Bearer abcdefghijklmnop 这种，password=12345678 也试了。",
            contact="someone@example.com",
        )
        lines = log_lines()
        self.assertTrue(lines)
        entry = lines[-1]
        self.assertEqual(entry["event"], "feedback")
        self.assertTrue(entry["maybe_secret"])
        self.assertTrue(entry["has_contact"])
        dump = json.dumps(lines, ensure_ascii=False)
        self.assertNotIn("Bearer", dump)
        self.assertNotIn("password=12345678", dump)
        self.assertNotIn("someone@example.com", dump)
        self.assertNotIn("登录这里好像有问题", dump)


class HttpTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = api.ThreadingHTTPServer(("127.0.0.1", 0), api.Handler)
        cls.port = cls.server.server_address[1]
        cls.thread = threading.Thread(target=cls.server.serve_forever)
        cls.thread.daemon = True
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()

    def post(self, body, headers=None):
        request = Request(
            "http://127.0.0.1:%d/feedback" % self.port,
            data=json.dumps(body).encode("utf-8"),
            headers=headers or {"Content-Type": "application/json"},
            method="POST",
        )
        return urlopen(request, timeout=5)

    def test_post_returns_id_and_cookie(self):
        response = self.post(payload(title="HTTP 层提交测试", body="通过真实 HTTP 请求提交的一条内容。"))
        self.assertEqual(response.status, 200)
        result = json.loads(response.read().decode("utf-8"))
        self.assertTrue(result["ok"])
        self.assertRegex(result["id"], r"^FB-\d{8}-[A-Z2-9]{4}$")
        self.assertTrue(re.search(r"xl_voter=[0-9a-f]{32}", response.headers.get("Set-Cookie", "")))

    def test_oversized_body_is_413(self):
        with self.assertRaises(HTTPError) as caught:
            self.post({"body": "x" * (api.FEEDBACK_MAX_BYTES + 100)})
        self.assertEqual(caught.exception.code, 413)

    def test_bad_json_is_400(self):
        request = Request(
            "http://127.0.0.1:%d/feedback" % self.port,
            data=b"{not json",
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with self.assertRaises(HTTPError) as caught:
            urlopen(request, timeout=5)
        self.assertEqual(caught.exception.code, 400)


if __name__ == "__main__":
    try:
        unittest.main(verbosity=2)
    finally:
        shutil.rmtree(STATS, ignore_errors=True)
