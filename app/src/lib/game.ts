import { useEffect, useState } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// Mirrors `GameStatus` in src-tauri/src/launch.rs
export interface GameStatus {
  running: boolean
  /** StarRail.exe, if Nous knows where the game is installed */
  path: string | null
}

/** Whether the game is running and where it is (event `game-status`). */
export function useGameStatus(): GameStatus | null {
  const [status, setStatus] = useState<GameStatus | null>(null)

  useEffect(() => {
    if (!isTauri()) return
    invoke<GameStatus>('get_game_status').then(setStatus).catch(console.error)
    const unlisten = listen<GameStatus>('game-status', (e) => setStatus(e.payload))
    return () => {
      unlisten.then((f) => f())
    }
  }, [])

  return status
}

/** Starts the game; false if the player declined the UAC prompt. */
export function launchGame(): Promise<boolean> {
  return invoke('launch_game')
}

/** Whether Nous starts with Windows in the tray (and pops up with the game). */
export function getAutostart(): Promise<boolean> {
  return invoke('get_autostart')
}

/** Turns autostart on or off; may show a UAC prompt. Resolves to the new state. */
export function setAutostart(enabled: boolean): Promise<boolean> {
  return invoke('set_autostart', { enabled })
}
