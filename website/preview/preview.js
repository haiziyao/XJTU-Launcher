(() => {
  const fine = matchMedia('(hover: hover) and (pointer: fine)').matches;
  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (!fine || reduced) return;

  const clamp = (v) => Math.min(1, Math.max(0, v));

  // Carousel: tilt and sheen follow the pointer over the active window.
  const stage = document.querySelector('#carousel .stage');
  if (stage) {
    let raf = 0;
    let win = null;
    let point = null;

    const clear = () => {
      stage.querySelectorAll('.window').forEach((w) => {
        w.style.removeProperty('--rx');
        w.style.removeProperty('--ry');
        w.style.removeProperty('--mx');
        w.style.removeProperty('--my');
      });
      win = null;
    };

    const apply = () => {
      raf = 0;
      if (!point) return;
      const slide = stage.querySelector('.slide.is-active');
      const next = slide && slide.querySelector('.window');
      if (!next) return;
      if (next !== win) {
        clear();
        win = next;
      }
      // Measure the slide, not the window: the window itself is tilted.
      const r = slide.getBoundingClientRect();
      const x = clamp((point.x - r.left) / r.width);
      const y = clamp((point.y - r.top) / r.height);
      win.style.setProperty('--ry', ((x - 0.5) * 6).toFixed(2) + 'deg');
      win.style.setProperty('--rx', ((0.5 - y) * 4).toFixed(2) + 'deg');
      win.style.setProperty('--mx', (x * 100).toFixed(1) + '%');
      win.style.setProperty('--my', (y * 100).toFixed(1) + '%');
    };

    stage.addEventListener('pointermove', (event) => {
      if (event.pointerType !== 'mouse') return;
      point = { x: event.clientX, y: event.clientY };
      if (!raf) raf = requestAnimationFrame(apply);
    });

    stage.addEventListener('pointerleave', () => {
      point = null;
      if (raf) cancelAnimationFrame(raf);
      raf = 0;
      clear();
    });
  }

  // Download cards: pass the pointer position to the edge light.
  const download = document.getElementById('download');
  if (download) {
    download.addEventListener('pointermove', (event) => {
      const card = event.target.closest && event.target.closest('.dl');
      if (!card) return;
      const r = card.getBoundingClientRect();
      card.style.setProperty('--mx', event.clientX - r.left + 'px');
      card.style.setProperty('--my', event.clientY - r.top + 'px');
    });
  }
})();
