(function () {
  "use strict";

  var GITHUB_RELEASES = "https://github.com/haiziyao/XJTU-Launcher/releases";

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

  function applyPill(data) {
    var pill = document.getElementById("pill-version");
    if (pill) pill.textContent = "v" + data.version + " · 开源 · MIT";
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

  function showFallback() {
    var cards = document.querySelectorAll(".dl");
    Array.prototype.forEach.call(cards, function (card) {
      card.href = GITHUB_RELEASES;
    });

    var details = document.querySelector(".checksums");
    if (details) details.hidden = true;

    var meta = document.getElementById("version-meta");
    if (!meta) return;
    var link = el("a", null, "GitHub Release");
    link.href = GITHUB_RELEASES;
    link.rel = "noopener";
    meta.replaceChildren(document.createTextNode("读取版本信息失败，请前往 "), link, document.createTextNode(" 下载"));
  }

  fetch("downloads.json", { cache: "no-cache" })
    .then(function (response) {
      if (!response.ok) throw new Error("HTTP " + response.status);
      return response.json();
    })
    .then(function (data) {
      applyPill(data);
      applyMeta(data);
      renderChecksums(data);
      return applyCards(data);
    })
    .catch(showFallback);
})();
