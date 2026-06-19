# Cryptainer — Dual Theme Design System

> Cryptainer is a security / cryptographic container app. The UI ships with **two completely distinct design systems** that the user toggles between. They do NOT share a token set — each theme has its own color palette, typography, layout grammar, motion language, and iconography. Switching the theme is a *re-skinning*, not a recolor.

---

## 1. Theme identity

| Theme | Codename | Mood | Vibe |
|---|---|---|---|
| Light | **PRISM** | Modern, elegant, soft | Apple-meets-Linear. Frosted glass, generous whitespace, calm motion. |
| Dark | **CIPHER** | Hacker, terminal, raw | Matrix / cyberdeck. Monospace, scanlines, glitch, ASCII frames. |

Default: **PRISM** (cryptainer is privacy software — most users open it in daylight).

---

## 2. PRISM (Light) — `data-theme="prism"`

### Color tokens
| Role | Hex | Use |
|---|---|---|
| `--prism-bg` | `#F7F5F2` | Page background (warm off-white) |
| `--prism-bg-elev` | `rgba(255,255,255,0.72)` | Glass card fill |
| `--prism-bg-elev-solid` | `#FFFFFF` | Solid fallback for glass |
| `--prism-ink` | `#0F172A` | Primary text |
| `--prism-ink-soft` | `#475569` | Secondary text |
| `--prism-ink-mute` | `#94A3B8` | Muted text / hints |
| `--prism-line` | `rgba(15,23,42,0.08)` | Hairline borders |
| `--prism-line-strong` | `rgba(15,23,42,0.14)` | Card borders |
| `--prism-accent` | `#7C5CFF` | Primary accent (electric violet) |
| `--prism-accent-2` | `#22D3EE` | Secondary accent (cyan) |
| `--prism-accent-soft` | `rgba(124,92,255,0.12)` | Tinted backgrounds |
| `--prism-success` | `#10B981` | Encrypted / OK |
| `--prism-danger` | `#EF4444` | Errors / delete |

### Typography
- **Heading**: `Outfit`, weight 600/700, tight tracking (-0.02em)
- **Body**: `Inter`, weight 400/500
- **Mono (for hashes/IDs only)**: `JetBrains Mono`
- Letter-spacing: -0.02em on H1/H2, normal on body

### Shape & motion
- Border-radius: 16px (cards), 12px (buttons), 8px (chips)
- Shadows: layered, soft, low-spread
  - `--prism-shadow-sm`: `0 1px 2px rgba(15,23,42,0.04), 0 1px 1px rgba(15,23,42,0.03)`
  - `--prism-shadow-md`: `0 8px 24px -8px rgba(15,23,42,0.12), 0 2px 6px rgba(15,23,42,0.05)`
  - `--prism-shadow-lg`: `0 24px 48px -16px rgba(15,23,42,0.18)`
- Easing: `cubic-bezier(0.22, 1, 0.36, 1)` (calm ease-out)
- Durations: 200ms (hover), 400ms (enter), 600ms (layout shift)
- Background: subtle radial gradient + faint grain SVG

### Layout grammar
- Centered, max-width 1200px
- Floating glass navbar (`top-4 left-4 right-4`, 16px from edges)
- 3-column responsive grid for vault cards
- Generous vertical rhythm (32-48px between sections)
- Cards: glass fill, hairline border, soft shadow, 1.25rem padding

### Card hover
`translateY(-4px)` + shadow upgrade + accent hairline glow. NO scale on body (avoid layout shift).

### Animations
- Page enter: fade + 8px slide-up, 400ms stagger
- Button hover: bg shift + inner glow, 200ms
- Card mount: opacity 0→1 + Y 12px→0, stagger 60ms

---

## 3. CIPHER (Dark) — `data-theme="cipher"`

### Color tokens
| Role | Hex | Use |
|---|---|---|
| `--cipher-bg` | `#000000` | Page background (true black) |
| `--cipher-bg-1` | `#050608` | Panel background |
| `--cipher-bg-2` | `#0A0D12` | Input/elevated panel |
| `--cipher-line` | `rgba(0,255,65,0.22)` | Neon hairline border |
| `--cipher-line-dim` | `rgba(0,255,65,0.08)` | Subtle divider |
| `--cipher-ink` | `#D1FFD7` | Primary text (soft green-white) |
| `--cipher-ink-soft` | `#7A8B7E` | Secondary text |
| `--cipher-ink-mute` | `#3F4A41` | Muted |
| `--cipher-accent` | `#00FF41` | Matrix green (primary) |
| `--cipher-accent-2` | `#FFB000` | Amber (secondary / warnings) |
| `--cipher-accent-3` | `#FF003C` | Magenta-red (danger / alerts) |
| `--cipher-glow` | `rgba(0,255,65,0.55)` | Outer glow |
| `--cipher-scan` | `rgba(0,255,65,0.04)` | Scanline color |

### Typography
- **Everything**: `JetBrains Mono`, weight 400/500/700
- Letter-spacing: 0 on body, +0.05em on UPPERCASE labels
- Text shadow on accent: `0 0 8px currentColor, 0 0 16px currentColor` (sparingly)

### Shape & motion
- Border-radius: **0** (sharp edges) — except 2px on toast/avatar
- Borders: 1px solid `--cipher-line` with `box-shadow: 0 0 12px var(--cipher-glow)` on focus/active
- Easing: `steps()` for glitch, `cubic-bezier(0.7, 0, 0.3, 1)` for slide
- Durations: 80ms (glitch), 150ms (hover), 300ms (panel open)

### Layout grammar
- Full-bleed, no rounded corner, terminal/CLI inspired
- Top bar: monospace status row with system metrics
- Vault view: **list layout** (not grid) — each container is a "row" in a terminal table
- ASCII frames: `┌─[ TITLE ]──...─┐` around panels
- Prompt prefix `>` before interactive elements
- Command-line visual cues: blinking caret, `[ OK ]`, `[ERR]`

### Card row (vault item)
```
┌─[ CTNR_ID ]────────────────────────────────────[ LOCKED ]─┐
│ > documents_2024         AES-256-GCM · 142.3 MB · 38 files │
│   created 2024-03-12 14:22   sha256:9f3a..b1              │
└────────────────────────────────────────────────────────────┘
```
Hover: border glow intensifies + accent `>` slides in.

### Animations
- **Glitch** on header: 2-keyframe skew + offset, 250ms, occasional
- **Scanline overlay** on whole app: `::before` with `repeating-linear-gradient` at 4% opacity, 8s scroll
- **Caret blink**: `▌` after interactive text, 1.06s steps(2)
- **Text scramble**: on mount, characters cycle 200ms before settling
- **Marquee** in top status bar: system status scrolling

---

## 4. Theme toggle

- Position: floating, top-right of the navbar (or top-bar in CIPHER)
- Two states: PRISM (sun icon) ↔ CIPHER (terminal icon)
- Transition: 600ms cross-fade + scale on all elements (View Transitions API where supported, fallback: opacity/transform)
- Persisted in `localStorage` (`cryptainer.theme`)
- No flash on reload: theme applied via inline `<script>` in `index.html` before React mount

---

## 5. Shared primitives (themed via CSS vars)

| Component | PRISM | CIPHER |
|---|---|---|
| Button | Rounded pill, gradient, soft shadow | Sharp, neon border, `> ` prefix, glow on hover |
| Input | Rounded, white fill, violet focus ring | Square, dark fill, blinking caret, green focus border |
| Card | Glass, 16px radius, soft shadow | ASCII frame, 0 radius, neon border + scanline |
| Modal | Centered, white, 24px radius, backdrop blur | Top-anchored "terminal window" with title bar `[ -o ] [ x ]` |
| Toast | Pill, soft shadow, slide-up | Top-center, monospace, `[ OK ]` / `[ERR]` prefix |
| Icon | Lucide stroke 1.5 | Lucide stroke 1 + neon text-shadow |

---

## 6. Anti-patterns

PRISM forbids: neon glow, dark backgrounds, monospace body, sharp corners, scanlines, glitch, terminal framing.
CIPHER forbids: rounded cards, soft pastels, glassmorphism, smooth gradient backgrounds, sans-serif body, emoji icons.

Both forbid: emoji as functional icon (use SVG), layout-shifting hovers, instant state changes, invisible focus, no `cursor: pointer` on interactive elements.
