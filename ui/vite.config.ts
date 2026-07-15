import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { resolve } from 'node:path';

// Three windows, three pages — mirrors the gpgui window model (main window,
// settings, connection manager). Tauri loads dist/<page>.html per window.
export default defineConfig({
  plugins: [
    svelte(),
    // Compile-time i18n: messages/{en,nl,fy}.json → tree-shaken functions in
    // src/paraglide (generated, git-ignored). Locale = user preference,
    // falling back to the base locale (en).
    paraglideVitePlugin({
      project: './project.inlang',
      outdir: './src/paraglide',
      strategy: ['preferredLanguage', 'baseLocale'],
    }),
  ],
  // Tauri expects a fixed dev port (tauri.conf.json build.devUrl).
  server: { port: 5173, strictPort: true },
  build: {
    target: 'safari15', // WebKitGTK baseline
    rollupOptions: {
      input: {
        index: resolve(import.meta.dirname, 'index.html'),
        settings: resolve(import.meta.dirname, 'settings.html'),
        manager: resolve(import.meta.dirname, 'manager.html'),
      },
    },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
});
