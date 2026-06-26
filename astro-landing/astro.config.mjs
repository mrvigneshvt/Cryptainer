// @ts-check
import { defineConfig } from 'astro/config';
import sitemap from '@astrojs/sitemap';

// Cryptainer marketing site — static, zero-hydration, SEO-first.
export default defineConfig({
  site: 'https://cryptainer.forked.online',
  integrations: [sitemap()],
  build: { inlineStylesheets: 'auto' },
  compressHTML: true,
});
