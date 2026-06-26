/* Sea ambience — rising bubbles + drifting plankton motes on a canvas.
   Light, GPU-friendly, honours prefers-reduced-motion. */

const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
const canvas = document.getElementById('bubbles');
if (canvas) {
  const ctx = canvas.getContext('2d');
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  let w = 0, h = 0, bubbles = [], motes = [];

  const rnd = (a, b) => a + Math.random() * (b - a);

  function spawnBubble(seed) {
    return {
      x: rnd(0, w),
      y: seed ? rnd(0, h) : h + rnd(0, 60),
      r: rnd(1, 4.5),
      sp: rnd(14, 42),          // px/sec upward
      drift: rnd(-10, 10),
      wob: rnd(0, Math.PI * 2),
      a: rnd(0.1, 0.4),
    };
  }
  function spawnMote() {
    return { x: rnd(0, w), y: rnd(0, h), r: rnd(0.4, 1.4), vx: rnd(-6, 6), vy: rnd(-4, 4), a: rnd(0.05, 0.22) };
  }

  function resize() {
    w = window.innerWidth; h = window.innerHeight;
    canvas.width = w * dpr; canvas.height = h * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const nB = Math.round(Math.min(46, w / 26));
    const nM = Math.round(Math.min(60, w / 22));
    bubbles = Array.from({ length: nB }, () => spawnBubble(true));
    motes = Array.from({ length: nM }, spawnMote);
  }

  let last = performance.now();
  function frame(now) {
    const dt = Math.min((now - last) / 1000, 0.05);
    last = now;
    ctx.clearRect(0, 0, w, h);

    // plankton motes
    for (const m of motes) {
      m.x += m.vx * dt; m.y += m.vy * dt;
      if (m.x < 0) m.x = w; if (m.x > w) m.x = 0;
      if (m.y < 0) m.y = h; if (m.y > h) m.y = 0;
      ctx.beginPath();
      ctx.fillStyle = `rgba(120, 255, 180, ${m.a})`;
      ctx.arc(m.x, m.y, m.r, 0, 6.283);
      ctx.fill();
    }
    // bubbles
    for (let i = 0; i < bubbles.length; i++) {
      const b = bubbles[i];
      b.y -= b.sp * dt;
      b.wob += dt * 2;
      b.x += (b.drift + Math.sin(b.wob) * 8) * dt;
      if (b.y + b.r < -10) bubbles[i] = spawnBubble(false);
      ctx.beginPath();
      ctx.strokeStyle = `rgba(0, 255, 90, ${b.a})`;
      ctx.lineWidth = 1;
      ctx.arc(b.x, b.y, b.r, 0, 6.283);
      ctx.stroke();
      // tiny highlight
      ctx.beginPath();
      ctx.fillStyle = `rgba(180, 255, 210, ${b.a * 0.7})`;
      ctx.arc(b.x - b.r * 0.3, b.y - b.r * 0.3, Math.max(0.4, b.r * 0.22), 0, 6.283);
      ctx.fill();
    }
    raf = requestAnimationFrame(frame);
  }

  let raf;
  resize();
  window.addEventListener('resize', resize, { passive: true });
  if (!reduced) raf = requestAnimationFrame(frame);
  else {
    // static sprinkle
    for (const m of motes) { ctx.beginPath(); ctx.fillStyle = `rgba(120,255,180,${m.a})`; ctx.arc(m.x, m.y, m.r, 0, 6.283); ctx.fill(); }
  }
}
