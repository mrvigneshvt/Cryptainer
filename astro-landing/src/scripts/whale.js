/* The Cipherwhale — a fixed, z-999 element that swims through the page as
   you scroll, carrying its sealed container and deflecting attacks off an
   encryption shield. Pure DOM + rAF. Honours prefers-reduced-motion. */

const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
const cw = document.getElementById('cw');
const attacksEl = document.getElementById('cwAttacks');
const statusEl = document.getElementById('cwStatus');

if (cw) {
  const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
  const lerp = (a, b, t) => a + (b - a) * t;
  const easeOut = (t) => 1 - Math.pow(1 - t, 3);

  let cwW = cw.offsetWidth;
  let cwH = cw.offsetHeight || cwW * 0.73;
  let vw = window.innerWidth, vh = window.innerHeight;

  function measure() { cwW = cw.offsetWidth; cwH = cw.offsetHeight || cwW * 0.73; vw = window.innerWidth; vh = window.innerHeight; }
  window.addEventListener('resize', measure, { passive: true });

  // ---- reduced motion: park the whale in the hero, no journey, no attacks ----
  if (reduced) {
    measure();
    cw.style.transform = `translate3d(${vw - cwW - 24}px, ${vh * 0.14}px, 0)`;
  } else {
    // ---------- scroll-driven path ----------
    let targetP = 0, curP = 0;
    function onScroll() {
      const max = document.documentElement.scrollHeight - vh;
      targetP = max > 0 ? clamp(window.scrollY / max, 0, 1) : 0;
    }
    window.addEventListener('scroll', onScroll, { passive: true });
    onScroll();

    // ---------- attacks ----------
    const LABELS = ['MITM', 'BRUTE-FORCE', 'MALWARE', 'SNIFFER', '0xBADC0DE', 'INJECTION', 'REPLAY', 'KEYLOG?'];
    let live = 0, lastSpawn = 0;

    function spawn(now) {
      if (live >= 3) return;
      live++;
      const el = document.createElement('span');
      el.className = 'atk';
      el.textContent = LABELS[(Math.random() * LABELS.length) | 0];
      attacksEl.appendChild(el);

      // angle biased to the front (right, 0 rad) so threats approach head-on
      const ang = (Math.random() * 2 - 1) * 1.15 + (Math.random() < 0.25 ? Math.PI : 0);
      const R0 = cwW * 0.62 + 60;          // spawn radius
      const Rs = cwW * 0.5;                 // shield contact radius
      const R2 = R0 * 1.5;                  // scatter radius
      const sx = Math.cos(ang) * R0, sy = Math.sin(ang) * R0 * 0.7;
      const cx = Math.cos(ang) * Rs, cy = Math.sin(ang) * Rs * 0.7;
      const scAng = ang + (Math.random() - 0.5) * 1.2;
      const ex = Math.cos(scAng) * R2, ey = Math.sin(scAng) * R2 * 0.7 - 40;
      const start = now, dur = 1300;
      let hit = false;

      function step(t) {
        const k = clamp((t - start) / dur, 0, 1);
        let dx, dy, op;
        if (k < 0.5) {                       // inbound to the shield
          const kk = k / 0.5;
          dx = lerp(sx, cx, easeOut(kk)); dy = lerp(sy, cy, easeOut(kk)); op = 0.95;
        } else {                             // deflected back out + fade
          if (!hit) { hit = true; flash(); el.classList.add('deflected'); }
          const kk = (k - 0.5) / 0.5;
          dx = lerp(cx, ex, easeOut(kk)); dy = lerp(cy, ey, easeOut(kk)); op = 0.9 * (1 - kk);
        }
        el.style.transform = `translate(calc(-50% + ${dx}px), calc(-50% + ${dy}px))`;
        el.style.opacity = op;
        if (k < 1) requestAnimationFrame(step);
        else { el.remove(); live--; }
      }
      requestAnimationFrame(step);
    }

    let flashUntil = 0;
    function flash() {
      cw.classList.add('is-hit');
      flashUntil = performance.now() + 200;
      if (statusEl) { statusEl.textContent = 'DEFLECTED ✓'; }
    }

    // ---------- main loop ----------
    function tick(now) {
      curP += (targetP - curP) * 0.08;       // smooth follow
      const p = curP;
      const t = now * 0.001;

      const mob = vw <= 760;
      // horizontal weave, biased to the right so it never sits on the
      // left-aligned text column for long
      const xFrac = clamp((mob ? 0.60 : 0.66) + (mob ? 0.26 : 0.22) * Math.sin(p * Math.PI * 2.2 + 0.5), 0.40, 0.86);
      // vertical arc: dives toward mid-page, rises near the end, plus a gentle bob.
      // On phones it rides lower so it never covers the hero headline.
      const yFrac = (mob ? 0.52 : 0.16) + (mob ? 0.20 : 0.32) * Math.sin(p * Math.PI) + 0.035 * Math.sin(t * 1.6);
      const x = xFrac * (vw - cwW);
      const y = clamp(yFrac, 0.05, mob ? 0.78 : 0.7) * (vh - cwH * 0.5);
      const rot = 5 * Math.sin(p * Math.PI * 2.2 + 0.5) + 2 * Math.sin(t * 1.6);

      cw.style.transform = `translate3d(${x.toFixed(1)}px, ${y.toFixed(1)}px, 0) rotate(${rot.toFixed(2)}deg)`;
      // keep the very top of the hero clean on phones — fade the whale in once scrolling
      cw.style.opacity = mob ? clamp((p - 0.02) / 0.08, 0, 0.85).toFixed(2) : '';

      // spawn attacks only mid-voyage (through the danger zone)
      if (p > 0.2 && p < 0.85 && now - lastSpawn > 1100) { lastSpawn = now; spawn(now); }

      if (flashUntil && now > flashUntil) {
        cw.classList.remove('is-hit');
        flashUntil = 0;
        if (statusEl) statusEl.textContent = 'SEALED';
      }
      requestAnimationFrame(tick);
    }
    requestAnimationFrame(tick);
  }
}
