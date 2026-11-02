import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  // Tauri expects a fixed port and watches src-tauri itself
  clearScreen: false,
  // expose TAURI_ENV_* (e.g. TAURI_ENV_DEBUG, set by `tauri dev` and `tauri build --debug`)
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  server: {
    port: 5173,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
})
