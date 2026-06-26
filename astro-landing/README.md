# Cryptainer — Landing Page

Marketing site for [Cryptainer](https://github.com/mrvigneshvt/cryptainer), the
offline encrypted container manager. Built with **Astro** for first-class SEO,
zero client-side framework hydration, and a static, instantly-cacheable output.

Live at **[cryptainer.forked.online](https://cryptainer.forked.online)**.
Designed & built by [forked.online](https://platform.forked.online).

## Design

Concept: **an encrypted whale carrying a sealed container safely through a
hostile digital sea.** Green-on-black palette (kept from Cryptainer's CIPHER
identity), with **Bricolage Grotesque** for the light display headlines and
**Instrument Sans** for body/UI; JetBrains Mono is reserved for ciphertext.

The signature element is the **Cipherwhale** — a hand-built SVG whale (`#cw`,
`z-index: 999`, `pointer-events: none`) that swims *through* the page as you
scroll: it weaves and dives along a path, carries an amber AES-256 container on
its back, and deflects incoming attacks (MITM, brute-force, malware…) off a
rotating encryption shield, flashing `SEALED → DEFLECTED`. Behind everything,
a fixed **sea** layer renders depth gradients, god rays, and a canvas of rising
bubbles + plankton. There's also a live, in-browser "encrypt this filename" demo.

All motion respects `prefers-reduced-motion` (the whale parks, attacks/bubbles
stop). On phones the whale rides low and stays hidden over the hero headline.

### Download links are placeholders

The five platform cards in the **Download** section (`downloads` array in
`src/pages/index.astro`) ship as placeholders: each `href` is `"#"` with a
`Soon` badge. Replace each `href` with the platform's release-asset URL and set
`soon: false` as builds are uploaded.

## Commands

```bash
npm install        # install dependencies
npm run dev        # local dev server  → http://localhost:4321
npm run build      # static build      → ./dist
npm run preview    # preview the build → http://localhost:4321
```

## SEO

- Canonical, Open Graph, and Twitter card meta
- JSON-LD structured data: `SoftwareApplication`, `Organization`, `FAQPage`
- `@astrojs/sitemap` → `sitemap-index.xml`, plus `public/robots.txt`
- Semantic HTML5, single `<h1>`, descriptive `alt` text, lazy-loaded images

Set the production domain in `astro.config.mjs` (`site`) before building.

## Structure

```
src/
  pages/index.astro        # the whole page + SEO head + JSON-LD + SVG whale
  styles/global.css        # design tokens, sea, whale, components
  scripts/sea.js           # canvas bubbles + plankton
  scripts/whale.js         # scroll-driven whale path + attack deflection
  scripts/effects.js       # scroll reveal + live encrypt demo
public/
  cryptainer-logo.png      # used for the OG/social card
  favicon.svg · robots.txt
```
