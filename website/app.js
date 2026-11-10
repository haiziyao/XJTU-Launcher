(function () {
  "use strict";

  var GITHUB_RELEASES = "https://github.com/haiziyao/XJTU-Launcher/releases";

  // Resolve relative to the page base so the same script works from /, /site/
  // and the preview copy (which uses <base href="../">).
  function assetUrl(path) {
    return new URL(path, document.baseURI).href;
  }

  function el(tag, className, text) {
    var node = document.createElement(tag);
    if (className) node.className = className;
    if (text) node.textContent = text;
    return node;
  }

  function formatSize(bytes) {
    if (!bytes || bytes < 0) return "";
    return (bytes / 1024 / 1024).toFixed(1) + " MB";
  }

  function formatCount(value) {
    if (typeof value !== "number" || !isFinite(value)) return "—";
    return value.toLocaleString("zh-CN");
  }

  function webglArch() {
    try {
      var gl = document.createElement("canvas").getContext("webgl");
      if (!gl) return "";
      var ext = gl.getExtension("WEBGL_debug_renderer_info");
      var renderer = String(
        gl.getParameter(ext ? ext.UNMASKED_RENDERER_WEBGL : gl.RENDERER) || ""
      );
      if (/Apple\s?M\d/i.test(renderer)) return "arm64";
      if (/Intel|AMD|Radeon/i.test(renderer)) return "x64";
    } catch (error) {
      /* no WebGL: fall back to the default below */
    }
    return "";
  }

  function detectMacArch() {
    var uaData = navigator.userAgentData;
    if (uaData && typeof uaData.getHighEntropyValues === "function") {
      return uaData
        .getHighEntropyValues(["architecture"])
        .then(function (values) {
          if (values.architecture === "arm") return "arm64";
          if (values.architecture === "x86") return "x64";
          return webglArch();
        })
        .catch(webglArch);
    }
    return Promise.resolve(webglArch());
  }

  function detectDevice() {
    var ua = navigator.userAgent;
    if (/Windows/i.test(ua)) {
      return Promise.resolve({ platform: "Windows", arch: "x64" });
    }
    if (/Mac OS X|Macintosh/i.test(ua)) {
      return detectMacArch().then(function (arch) {
        return { platform: "macOS", arch: arch || "arm64" };
      });
    }
    return Promise.resolve({ platform: "", arch: "" });
  }

  function findFile(files, platform, arch) {
    for (var i = 0; i < files.length; i += 1) {
      if (files[i].platform === platform && files[i].arch === arch) return files[i];
    }
    return null;
  }

  function applyCards(data) {
    var cards = document.querySelectorAll(".dl");

    return detectDevice().then(function (current) {
      Array.prototype.forEach.call(cards, function (card) {
        var platform = card.getAttribute("data-platform");
        var arch = card.getAttribute("data-arch");
        var file = findFile(data.files, platform, arch);
        var small = card.querySelector("small");

        if (!file) {
          card.href = data.releases_url || GITHUB_RELEASES;
          return;
        }

        card.href = data.base_url + file.name;
        if (small) {
          small.textContent =
            platform === "Windows"
              ? "10 / 11 · " + formatSize(file.size)
              : file.arch_label + " · " + formatSize(file.size);
        }

        if (current.platform === platform && current.arch === arch) {
          card.classList.add("is-current");
          card.style.order = "-1";
          if (!card.querySelector(".dl-badge")) {
            card.prepend(el("span", "dl-badge", "推荐"));
          }
        }
      });
    });
  }

  function applyTags(data) {
    var version = document.getElementById("pill-version");
    if (version) version.textContent = "v" + data.version;

    var stage = document.getElementById("release-stage");
    if (!stage) return;
    var major = parseInt(String(data.version), 10);
    var stable = Number.isFinite(major) && major >= 1;
    stage.textContent = stable ? "正式版" : "预览版";
    stage.classList.toggle("is-stable", stable);
  }

  function initTitle() {
    var title = document.getElementById("hero-title");
    if (!title) return;
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    var nodes = Array.prototype.slice.call(title.childNodes);
    var frag = document.createDocumentFragment();
    var index = 0;
    var step = 55;

    nodes.forEach(function (node) {
      if (node.nodeType === 3) {
        Array.prototype.forEach.call(node.textContent, function (ch) {
          if (ch === " " || ch === "\n" || ch === "\t") {
            frag.append(document.createTextNode(ch));
            return;
          }
          var span = document.createElement("span");
          span.className = "char";
          span.textContent = ch;
          span.style.setProperty("--d", index * step + "ms");
          index += 1;
          frag.append(span);
        });
      } else if (node.nodeName === "BR") {
        frag.append(document.createElement("br"));
      }
    });

    var caret = document.createElement("span");
    caret.className = "caret";
    caret.setAttribute("aria-hidden", "true");
    frag.append(caret);

    title.replaceChildren(frag);

    var total = index * step + 420;
    window.setTimeout(function () {
      caret.classList.add("show");
    }, total);
    window.setTimeout(function () {
      caret.classList.add("done");
    }, total + 6000);
  }

  function applyMeta(data) {
    var meta = document.getElementById("version-meta");
    if (!meta) return;

    var link = el("a", null, "GitHub Release");
    link.href = data.releases_url || GITHUB_RELEASES;
    link.rel = "noopener";

    meta.replaceChildren(
      document.createTextNode(
        "当前版本 " + data.version + " · " + data.date + " · 安装包同时发布在 "
      ),
      link
    );
  }

  function renderChecksums(data) {
    var list = document.getElementById("checksum-list");
    if (!list) return;
    list.replaceChildren();

    data.files.forEach(function (file) {
      var row = el("div", "cs-row");
      row.append(el("span", "name", file.name));
      row.append(el("code", null, file.sha256));

      var copy = el("button", "cs-copy", "复制");
      copy.type = "button";
      copy.setAttribute("aria-label", "复制 " + file.name + " 的 SHA256");
      copy.addEventListener("click", function () {
        copyText(file.sha256).then(
          function () {
            copy.textContent = "已复制";
            window.setTimeout(function () {
              copy.textContent = "复制";
            }, 1600);
          },
          function () {
            copy.textContent = "复制失败";
          }
        );
      });

      row.append(copy);
      list.append(row);
    });
  }

  function copyText(text) {
    if (navigator.clipboard && window.isSecureContext) {
      return navigator.clipboard.writeText(text);
    }
    return new Promise(function (resolve, reject) {
      var area = document.createElement("textarea");
      area.value = text;
      area.setAttribute("readonly", "");
      area.style.position = "fixed";
      area.style.opacity = "0";
      document.body.append(area);
      area.select();
      var ok = false;
      try {
        ok = document.execCommand("copy");
      } catch (error) {
        ok = false;
      }
      area.remove();
      ok ? resolve() : reject(new Error("copy failed"));
    });
  }

  function setLikeCount(button, value) {
    if (typeof value !== "number" || !isFinite(value)) return;
    var num = button.querySelector(".like-num");
    if (num) num.textContent = formatCount(value);
  }

  function markVoted(button) {
    button.classList.add("is-voted");
    button.setAttribute("aria-pressed", "true");
    var label = button.querySelector(".like-label");
    if (label) label.textContent = "人支持 · 你已投";
  }

  function markUnvoted(button) {
    button.classList.remove("is-voted");
    button.setAttribute("aria-pressed", "false");
    var label = button.querySelector(".like-label");
    if (label) label.textContent = "人支持";
  }

  function initLikes() {
    var buttons = document.querySelectorAll(".like");
    if (!buttons.length) return;

    Array.prototype.forEach.call(buttons, function (button) {
      var key = button.getAttribute("data-key");

      button.addEventListener("click", function () {
        // The server toggles: clicking a voted button cancels that vote.
        if (button.dataset.busy) return;
        button.dataset.busy = "1";
        button.classList.remove("is-error");

        fetch(assetUrl("api/like"), {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ key: key })
        })
          .then(function (response) {
            if (!response.ok) throw new Error("HTTP " + response.status);
            return response.json();
          })
          .then(function (result) {
            setLikeCount(button, result.count);
            if (result.voted) {
              markVoted(button);
            } else {
              markUnvoted(button);
            }
          })
          .catch(function () {
            button.classList.add("is-error");
            var label = button.querySelector(".like-label");
            if (!label) return;
            label.textContent = "暂时不可用";
            window.setTimeout(function () {
              label.textContent = button.classList.contains("is-voted")
                ? "人支持 · 你已投"
                : "人支持";
              button.classList.remove("is-error");
            }, 2400);
          })
          .then(function () {
            delete button.dataset.busy;
          });
      });
    });
  }

  function loadStats() {
    fetch(assetUrl("api/stats"), { cache: "no-store" })
      .then(function (response) {
        if (!response.ok) throw new Error("HTTP " + response.status);
        return response.json();
      })
      .then(function (data) {
        var chip = document.getElementById("download-stat");
        var total = data && data.downloads ? data.downloads.total : null;
        if (chip && typeof total === "number") {
          var value = document.getElementById("download-total");
          if (value) value.textContent = formatCount(total);
          chip.hidden = false;
        }

        var likes = (data && data.likes) || {};
        var mine = data && data.voted;
        Array.prototype.forEach.call(document.querySelectorAll(".like"), function (button) {
          var key = button.getAttribute("data-key");
          setLikeCount(button, likes[key]);
          // The server knows which directions this browser voted for, so the
          // button state survives counter resets and is independent of the
          // total count.
          if (!mine || typeof mine[key] !== "boolean") return;
          if (mine[key]) {
            markVoted(button);
          } else {
            markUnvoted(button);
          }
        });
      })
      .catch(function () {
        /* 接口不可用时隐藏下载量，不显示未经统计的数字 */
      });
  }

  function showFallback() {
    var cards = document.querySelectorAll(".dl");
    Array.prototype.forEach.call(cards, function (card) {
      card.href = GITHUB_RELEASES;
    });

    var details = document.querySelector(".checksums");
    if (details) details.hidden = true;

    // The static HTML cannot know the current version; do not leave a stale placeholder on screen.
    var version = document.getElementById("pill-version");
    if (version) version.textContent = "v—";

    var meta = document.getElementById("version-meta");
    if (!meta) return;
    var link = el("a", null, "GitHub Release");
    link.href = GITHUB_RELEASES;
    link.rel = "noopener";
    meta.replaceChildren(document.createTextNode("读取版本信息失败，请前往 "), link, document.createTextNode(" 下载"));
  }

  fetch(assetUrl("downloads.json"), { cache: "no-cache" })
    .then(function (response) {
      if (!response.ok) throw new Error("HTTP " + response.status);
      return response.json();
    })
    .then(function (data) {
      applyTags(data);
      applyMeta(data);
      renderChecksums(data);
      return applyCards(data);
    })
    .catch(showFallback);

  initTitle();
  initLikes();
  loadStats();
})();
