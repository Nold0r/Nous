import react from '@vitejs/plugin-react'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  // modules/<id>/ import the shell's building blocks as @app/ui, @app/lib/...
  resolve: { alias: { '@app': fileURLToPath(new URL('./src', import.meta.url)) } },
  // Tauri expects a fixed port and watches src-tauri itself
  clearScreen: false,
  // expose TAURI_ENV_* (e.g. TAURI_ENV_DEBUG, set by `tauri dev` and `tauri build --debug`)
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  server: {
    port: 5173,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
    // CHANGELOG.md lives at the repo root (lib/changelog.ts imports it)
    fs: { allow: ['..'] },
  },
})
