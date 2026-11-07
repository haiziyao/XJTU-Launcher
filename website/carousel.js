(() => {
  const root = document.getElementById("carousel");
  if (!root) return;

  const slides = [...root.querySelectorAll(".slide")];
  const dotsBox = document.getElementById("dots");
  const glow = document.getElementById("glow");
  const caption = document.getElementById("caption");
  const tag = document.getElementById("tag");
  const ctitle = document.getElementById("ctitle");
  const cdesc = document.getElementById("cdesc");

  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const INTERVAL = 4600;

  let index = 0;
  let timer = null;
  let paused = false;
  let captionTimer = null;

  const dots = slides.map((slide, i) => {
    const dot = document.createElement("button");
    dot.className = "dot";
    dot.type = "button";
    dot.setAttribute("role", "tab");
    dot.setAttribute("aria-label", slide.dataset.tag || `第 ${i + 1} 张`);
    dot.addEventListener("click", () => go(i, true));
    dotsBox.appendChild(dot);
    return dot;
  });

  slides.forEach((slide, i) => {
    slide.addEventListener("click", () => {
      if (i !== index) go(i, true);
    });
  });

  function relative(i, active) {
    const count = slides.length;
    let d = i - active;
    if (d > count / 2) d -= count;
    if (d < -count / 2) d += count;
    return d;
  }

  function paint(active, first) {
    const slide = slides[active];
    const accent = slide.dataset.accent || "#7c5cff";
    root.style.setProperty("--accent", accent);

    slides.forEach((s, i) => {
      const d = relative(i, active);
      s.classList.toggle("is-active", d === 0);
      s.classList.toggle("is-prev", d === -1);
      s.classList.toggle("is-next", d === 1);
      s.setAttribute("aria-hidden", d === 0 ? "false" : "true");
    });

    dots.forEach((dot, i) => {
      dot.classList.toggle("is-active", i === active);
      dot.setAttribute("aria-selected", i === active ? "true" : "false");
    });

    glow.style.background = `radial-gradient(circle at 50% 55%, ${accent}66, transparent 62%)`;
    updateCaption(slide, first);
  }

  function updateCaption(slide, first) {
    clearTimeout(captionTimer);
    const fill = () => {
      tag.textContent = slide.dataset.tag || "";
      ctitle.textContent = slide.dataset.title || "";
      cdesc.textContent = slide.dataset.desc || "";
      caption.classList.remove("swap");
    };
    if (first) return fill();
    caption.classList.add("swap");
    captionTimer = setTimeout(fill, 180);
  }

  function go(next, manual) {
    const count = slides.length;
    next = ((next % count) + count) % count;
    if (next === index) return;
    if (manual) restart();
    index = next;
    paint(index, false);
  }

  function restart() {
    clearInterval(timer);
    if (reduced || paused) return;
    timer = setInterval(() => go(index + 1), INTERVAL);
  }

  function pause() {
    if (paused) return;
    paused = true;
    clearInterval(timer);
    root.classList.add("is-paused");
  }

  function resume() {
    if (!paused) return;
    paused = false;
    root.classList.remove("is-paused");
    restart();
  }

  root.querySelector("#next").addEventListener("click", () => go(index + 1, true));
  root.querySelector("#prev").addEventListener("click", () => go(index - 1, true));

  document.addEventListener("visibilitychange", () => (document.hidden ? pause() : resume()));

  root.addEventListener("keydown", (event) => {
    if (event.key === "ArrowRight") {
      event.preventDefault();
      go(index + 1, true);
    }
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      go(index - 1, true);
    }
  });

  // Drag / swipe.
  let startX = null;
  root.addEventListener("pointerdown", (event) => {
    if (event.pointerType !== "touch") return;
    startX = event.clientX;
  });
  root.addEventListener("pointerup", (event) => {
    if (startX === null) return;
    const delta = event.clientX - startX;
    startX = null;
    if (Math.abs(delta) < 44) return;
    if (delta < 0) go(index + 1, true);
    else go(index - 1, true);
  });

  const hash = Number.parseInt(location.hash.replace("#slide-", ""), 10);
  if (hash >= 1 && hash <= slides.length) index = hash - 1;
  paint(index, true);
  restart();
})();
