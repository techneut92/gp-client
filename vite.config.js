import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { resolve } from 'node:path';

// Three windows, three pages — mirrors the gpgui window model (main window,
// settings, connection manager). Tauri loads dist/<page>.html per window.
export default defineConfig({
  plugins: [svelte()],
  // Tauri expects a fixed dev port (tauri.conf.json build.devUrl).
  server: { port: 5173, strictPort: true },
  build: {
    target: 'safari15', // WebKitGTK baseline
    rollupOptions: {
      input: {
        index: resolve(__dirname, 'index.html'),
        settings: resolve(__dirname, 'settings.html'),
        manager: resolve(__dirname, 'manager.html'),
      },
    },
  },
  // Never pull in the Tauri CLI's env vars as defines.
  envPrefix: ['VITE_', 'TAURI_ENV_'],
});
