import { useState } from 'react'
import { Play } from 'lucide-react'
import { launchGame, useGameStatus } from '../../lib/game'

/** Starts the game from Nous, so Nous is already running when the player logs in. */
export function PlayButton() {
  const game = useGameStatus()
  const [starting, setStarting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  if (!game) return null

  const play = () => {
    setStarting(true)
    setError(null)
    launchGame()
      .catch((e) => setError(String(e)))
      // the button flips to "Игра запущена" when the watcher sees the process
      .finally(() => setTimeout(() => setStarting(false), 5000))
  }

  const label = game.running ? 'Игра запущена' : starting ? 'Запуск…' : 'Играть'
  const hint = error ?? (game.path ? undefined : 'Путь к игре не найден. Запустите игру один раз, и Nous его запомнит.')

  return (
    <div className="play">
      <button
        type="button"
        className="play__button"
        onClick={play}
        disabled={game.running || starting || !game.path}
        title={game.path ?? undefined}
      >
        <Play size={16} />
        {label}
      </button>
      {hint && !game.running && <span className="play__hint">{hint}</span>}
    </div>
  )
}
