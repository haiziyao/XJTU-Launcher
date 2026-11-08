(() => {
  const root = document.getElementById("carousel");
  if (!root) return;

  const slides = [...root.querySelectorAll(".slide")];
  const dotsBox = document.getElementById("dots");
  const glow = document.getElementById("glow");
  const parallax = document.getElementById("parallax");
  const caption = document.getElementById("caption");
  const tag = document.getElementById("tag");
  const ctitle = document.getElementById("ctitle");
  const cdesc = document.getElementById("cdesc");

  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const INTERVAL = 5200;

  let index = 0;
  let timer = null;
  let paused = false;
  let busy = false;
  let captionTimer = null;

  const dots = slides.map((slide, i) => {
    const dot = document.createElement("button");
    dot.className = "dot";
    dot.type = "button";
    dot.setAttribute("role", "tab");
    dot.setAttribute("aria-label", slide.dataset.tag || `第 ${i + 1} 张`);
    dot.addEventListener("click", () => go(i, i > index ? 1 : -1, true));
    dotsBox.appendChild(dot);
    return dot;
  });

  function paint(i, first) {
    const slide = slides[i];
    const accent = slide.dataset.accent || "#7c5cff";
    root.style.setProperty("--accent", accent);
    slides.forEach((s, k) => {
      s.classList.toggle("is-active", k === i);
      s.setAttribute("aria-hidden", k === i ? "false" : "true");
    });
    dots.forEach((d, k) => {
      d.classList.toggle("is-active", k === i);
      d.setAttribute("aria-selected", k === i ? "true" : "false");
    });
    glow.style.background = `radial-gradient(circle at 50% 50%, ${accent}4d, transparent 62%)`;
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
    captionTimer = setTimeout(fill, 200);
  }

  function go(next, dir, manual) {
    if (next === index || busy || next < 0 || next >= slides.length) return;
    if (manual) restart();

    const out = slides[index];
    const inn = slides[next];
    busy = true;

    out.classList.remove("is-active");
    out.classList.add(dir > 0 ? "leaving-left" : "leaving-right");

    inn.classList.remove("leaving-left", "leaving-right");
    inn.style.transition = "none";
    inn.style.transform = `translateX(${dir > 0 ? 64 : -64}px) scale(.96)`;
    inn.style.opacity = "0";
    void inn.offsetWidth;
    inn.style.transition = "";
    inn.style.transform = "";
    inn.style.opacity = "";

    index = next;
    paint(next, false);

    setTimeout(() => {
      out.classList.remove("leaving-left", "leaving-right");
      busy = false;
    }, reduced ? 220 : 720);
  }

  function restart() {
    clearInterval(timer);
    if (reduced || paused) return;
    timer = setInterval(() => go((index + 1) % slides.length, 1), INTERVAL);
  }

  function pause() {
    paused = true;
    clearInterval(timer);
  }

  function resume() {
    paused = false;
    restart();
  }

  root.querySelector("#next").addEventListener("click", () => go((index + 1) % slides.length, 1, true));
  root.querySelector("#prev").addEventListener("click", () => go((index - 1 + slides.length) % slides.length, -1, true));

  root.addEventListener("mouseenter", pause);
  root.addEventListener("mouseleave", () => {
    resume();
    parallax.style.transform = "";
  });
  root.addEventListener("focusin", pause);
  root.addEventListener("focusout", resume);
  document.addEventListener("visibilitychange", () => (document.hidden ? pause() : resume()));

  root.addEventListener("keydown", (event) => {
    if (event.key === "ArrowRight") go((index + 1) % slides.length, 1, true);
    if (event.key === "ArrowLeft") go((index - 1 + slides.length) % slides.length, -1, true);
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
    if (delta < 0) go((index + 1) % slides.length, 1, true);
    else go((index - 1 + slides.length) % slides.length, -1, true);
  });

  // Subtle pointer parallax, desktop only.
  if (!reduced && matchMedia("(hover: hover)").matches) {
    root.addEventListener("pointermove", (event) => {
      if (event.pointerType !== "mouse") return;
      const rect = root.getBoundingClientRect();
      const dx = (event.clientX - rect.left) / rect.width - 0.5;
      const dy = (event.clientY - rect.top) / rect.height - 0.5;
      parallax.style.transform = `translate3d(${dx * 10}px, ${dy * 6}px, 0)`;
    });
  }

  const hash = Number.parseInt(location.hash.replace("#slide-", ""), 10);
  if (hash >= 1 && hash <= slides.length) index = hash - 1;
  paint(index, true);
  restart();
})();
