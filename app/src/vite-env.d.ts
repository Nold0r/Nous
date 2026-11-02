/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** 'true' in `tauri dev` and `tauri build --debug` (see envPrefix in vite.config.ts) */
  readonly TAURI_ENV_DEBUG?: string
  /** 'true' in `npm run tauri:user`: dev build that looks like a release (no dev-only screens) */
  readonly VITE_USER_MODE?: string
}
