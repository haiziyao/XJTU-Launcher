(function () {
  "use strict";

  // Version and date come from the same manifest the homepage reads, so a
  // release does not need this page edited by hand. On failure the placeholders
  // stay empty instead of showing a version nobody has verified.
  (function fillVersion() {
    var versions = Array.prototype.slice.call(document.querySelectorAll("[data-qa-version]"));
    var dates = Array.prototype.slice.call(document.querySelectorAll("[data-qa-date]"));
    if (!versions.length && !dates.length) return;

    fetch(new URL("../../downloads.json", document.baseURI).href, { cache: "no-cache" })
      .then(function (response) {
        if (!response.ok) throw new Error("HTTP " + response.status);
        return response.json();
      })
      .then(function (data) {
        var version = String((data && data.version) || "");
        if (!/^\d+\.\d+\.\d+$/.test(version)) return;
        versions.forEach(function (node) {
          node.textContent = "v" + version;
        });
        var date = String(data.date || "");
        if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return;
        dates.forEach(function (node) {
          node.textContent = date;
        });
      })
      .catch(function () {
        /* keep the placeholder */
      });
  })();

  var nav = document.querySelector(".side-nav");
  if (!nav) return;

  var links = Array.prototype.slice.call(nav.querySelectorAll(".side-link, .side-cat"));
  var questions = Array.prototype.slice.call(document.querySelectorAll(".qa details"));
  var groups = Array.prototype.slice.call(document.querySelectorAll(".qa-group"));
  var panel = document.querySelector(".side-box");

  var current = null;

  function linkFor(id) {
    return nav.querySelector('a[href="#' + id + '"]');
  }

  function setActive(link) {
    if (!link || link === current) return;
    current = link;
    links.forEach(function (a) { a.classList.toggle("is-active", a === link); });
  }

  function openQuestion(el) {
    if (el && el.tagName === "DETAILS") el.open = true;
  }

  function scrollTo(el) {
    if (!el) return;
    var reduce = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    el.scrollIntoView({ block: "start", behavior: reduce ? "auto" : "smooth" });
  }

  // Clicking any entry marks it active, opens the target question and
  // (on the collapsed mobile panel) closes the directory.
  nav.addEventListener("click", function (event) {
    var link = event.target.closest("a");
    if (!link || !nav.contains(link)) return;

    var href = link.getAttribute("href") || "";
    if (href.charAt(0) !== "#") return;

    event.preventDefault();
    setActive(link);

    var id = href.slice(1);
    var target = document.getElementById(id);
    openQuestion(target);
    scrollTo(target);
    if (window.history && window.history.pushState) {
      window.history.pushState(null, "", href);
    }

    if (window.innerWidth <= 900 && panel) panel.open = false;
  });

  // Scroll spy: highlight the question currently passing under the header,
  // falling back to its category. At the very top nothing matches, so the
  // default entry stays "常见问题".
  var pending = false;
  function updateSpy() {
    pending = false;
    var threshold = 130;
    var groupLink = null;

    groups.forEach(function (group) {
      if (group.getBoundingClientRect().top <= threshold) groupLink = linkFor(group.id);
    });

    if (!groupLink) return setActive(linkFor("cat-basic"));

    var questionLink = null;
    questions.forEach(function (item) {
      if (item.getBoundingClientRect().top <= threshold) questionLink = linkFor(item.id);
    });

    setActive(questionLink || groupLink);
  }

  function scheduleSpy() {
    if (pending) return;
    pending = true;
    window.requestAnimationFrame(updateSpy);
  }

  window.addEventListener("scroll", scheduleSpy, { passive: true });
  window.addEventListener("resize", scheduleSpy);

  // Deep links: /doc/qa/#q-macos opens that answer and highlights it.
  function applyHash() {
    var id = (window.location.hash || "").slice(1);
    if (!id) return;
    var target = document.getElementById(id);
    if (!target) return;
    openQuestion(target);
    setActive(linkFor(id));
  }

  window.addEventListener("hashchange", applyHash);

  if (window.innerWidth <= 900 && panel) panel.open = false;
  applyHash();
  setActive(linkFor("cat-basic"));
})();
