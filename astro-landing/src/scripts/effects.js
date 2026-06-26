/* UI effects: scroll reveal + the live encrypt demo. */

const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

/* ---------- scroll reveal ---------- */
function reveals() {
  const els = document.querySelectorAll('.reveal');
  if (reduced || !('IntersectionObserver' in window)) { els.forEach((e) => e.classList.add('in')); return; }
  const io = new IntersectionObserver((entries) => {
    entries.forEach((e) => { if (e.isIntersecting) { e.target.classList.add('in'); io.unobserve(e.target); } });
  }, { threshold: 0.12, rootMargin: '0px 0px -8% 0px' });
  els.forEach((e) => io.observe(e));
}

/* ---------- live encrypt demo ---------- */
function demo() {
  const input = document.getElementById('demoInput');
  const cipherEl = document.getElementById('demoCipher');
  const sizeEl = document.getElementById('demoSize');
  const nonceEl = document.getElementById('demoNonce');
  const tagEl = document.getElementById('demoTag');
  if (!input) return;

  const HEX = '0123456789abcdef';
  // deterministic pseudo-random from a string seed (FNV-1a + xorshift)
  function seeded(str, n) {
    let h = 2166136261;
    for (let i = 0; i < str.length; i++) { h ^= str.charCodeAt(i); h = Math.imul(h, 16777619); }
    let out = '';
    for (let i = 0; i < n; i++) { h ^= h << 13; h ^= h >>> 17; h ^= h << 5; out += HEX[(h >>> (i % 24)) & 15]; }
    return out;
  }
  const grouped = (hex) => hex.match(/.{1,2}/g).join(' ');

  let raf, frame = 0;
  function render() {
    const name = input.value.trim() || 'tax_return_2024.pdf';
    const seed = name + name.length;
    const bytes = Math.max(48, name.length * 12 + 96);
    const full = seeded(seed, bytes);
    cancelAnimationFrame(raf);
    frame = 0;
    function tickFrame() {
      const p = reduced ? 1 : Math.min(frame / 26, 1);
      const reveal = Math.floor(p * full.length);
      let out = '';
      for (let i = 0; i < full.length; i++) out += i < reveal ? full[i] : HEX[(Math.random() * 16) | 0];
      cipherEl.textContent = grouped(out);
      if (p < 1) { frame++; raf = requestAnimationFrame(tickFrame); }
    }
    tickFrame();
    sizeEl.textContent = bytes / 2 + ' B';
    nonceEl.textContent = seeded('n' + seed, 24);
    tagEl.textContent = seeded('t' + seed, 32);
  }

  input.addEventListener('input', render);
  render();
}

reveals();
demo();
