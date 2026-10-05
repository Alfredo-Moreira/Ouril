import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import react from '@vitejs/plugin-react';
import { VitePWA } from 'vite-plugin-pwa';
import { defineConfig, type Plugin } from 'vitest/config';

// Guest-first: the app never calls the API unless the player signs in (ADR 0014, ADR 0011).
// In Docker the dev server proxies /v1 to the `server` service (compose.yaml).
const apiTarget = process.env.VITE_API_PROXY_TARGET ?? 'http://localhost:8080';
const here = dirname(fileURLToPath(import.meta.url));
const pkg = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8')) as {
  version: string;
};

/**
 * `virtual:music-tracks`: the URLs of every MP3 in public/audio/game/ (sorted), so adding a
 * game track is just dropping the file in. The dev server reloads when files come and go.
 */
function musicTracks(): Plugin {
  const dir = resolve(here, 'public/audio/game');
  const id = 'virtual:music-tracks';
  const resolved = '\0' + id;
  const list = () =>
    existsSync(dir)
      ? readdirSync(dir)
          .filter((f) => /\.mp3$/i.test(f))
          .sort()
          .map((f) => `/audio/game/${encodeURIComponent(f)}`)
      : [];
  return {
    name: 'ouril-music-tracks',
    resolveId: (source) => (source === id ? resolved : undefined),
    load: (moduleId) =>
      moduleId === resolved ? `export default ${JSON.stringify(list())};` : undefined,
    configureServer(server) {
      server.watcher.add(dir);
      const refresh = (file: string) => {
        if (!file.startsWith(dir)) return;
        const mod = server.moduleGraph.getModuleById(resolved);
        if (mod) server.moduleGraph.invalidateModule(mod);
        server.ws.send({ type: 'full-reload' });
      };
      server.watcher.on('add', refresh);
      server.watcher.on('unlink', refresh);
    },
  };
}

/** Dev only: regenerate src/generated/i18n when shared/i18n/*.json changes. */
function i18nWatch(): Plugin {
  const sharedDir = resolve(here, '../../shared/i18n');
  return {
    name: 'ouril-i18n-watch',
    apply: 'serve',
    configureServer(server) {
      server.watcher.add(sharedDir);
      server.watcher.on('change', (file) => {
        if (!file.startsWith(sharedDir) || !file.endsWith('.json')) return;
        try {
          execFileSync('node', [resolve(here, 'scripts/gen-i18n.mjs')], { stdio: 'inherit' });
        } catch {
          // The script already printed what's wrong.
        }
      });
    },
  };
}

export default defineConfig({
  plugins: [
    react(),
    i18nWatch(),
    musicTracks(),
    VitePWA({
      registerType: 'autoUpdate',
      injectRegister: false,
      manifest: {
        name: 'Ouril',
        short_name: 'Ouril',
        description: 'The Cape Verdean seed-sowing game. Play offline against the computer.',
        lang: 'en',
        start_url: '/',
        scope: '/',
        display: 'standalone',
        background_color: '#f6efe3',
        theme_color: '#7a4a22',
        icons: [
          { src: '/icons/icon-192.png', sizes: '192x192', type: 'image/png' },
          { src: '/icons/icon-512.png', sizes: '512x512', type: 'image/png' },
          { src: '/icons/icon-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
          { src: '/icons/icon.svg', sizes: 'any', type: 'image/svg+xml' },
        ],
      },
      workbox: {
        // App shell + the WASM core, so the game works offline after the first load.
        globPatterns: ['**/*.{js,css,html,wasm,svg,png,avif,webmanifest}'],
        maximumFileSizeToCacheInBytes: 5 * 1024 * 1024,
        navigateFallback: '/index.html',
        // Never serve the app shell for API calls, and never cache API responses.
        navigateFallbackDenylist: [/^\/v1\//, /^\/healthz/, /^\/audio\//],
        cleanupOutdatedCaches: true,
        // Background music (src/audio/music.ts): its own cache, warmed in the background after
        // the first visit so it plays offline. Range requests are supported (Safari streams
        // <audio> in ranges), and only real MP3 responses are stored, never an error page.
        runtimeCaching: [
          {
            urlPattern: ({ url }) => url.pathname.startsWith('/audio/'),
            handler: 'CacheFirst',
            options: {
              cacheName: 'ouril-music',
              rangeRequests: true,
              cacheableResponse: { statuses: [200], headers: { 'Content-Type': 'audio/mpeg' } },
              expiration: { maxEntries: 60 },
            },
          },
        ],
      },
    }),
  ],
  define: {
    'import.meta.env.VITE_APP_VERSION': JSON.stringify(pkg.version),
  },
  worker: {
    format: 'es',
  },
  server: {
    host: true,
    port: 5173,
    strictPort: true,
    proxy: {
      '/v1': { target: apiTarget, changeOrigin: false },
    },
  },
  build: {
    target: 'es2022',
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.test.{ts,tsx}'],
    // Tests cover the unreleased features too; `features.test.tsx` checks they're hidden when off.
    env: {
      VITE_FEATURE_ACCOUNTS: 'true',
      VITE_FEATURE_STATS: 'true',
      // Telemetry "configured" so the consent prompt is tested; it still never sends anything
      // (GuestNetwork.test.tsx), and features.test.tsx covers the MVP without it.
      VITE_SENTRY_DSN: 'https://public@telemetry.invalid/1',
    },
  },
});
