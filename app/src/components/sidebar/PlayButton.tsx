import { useState } from 'react'
import { Play } from 'lucide-react'
import { launchGame, useGameStatus } from '../../lib/game'

/** Starts the game from Nous, so Nous is already running when the player logs in. */
export function PlayButton() {
  const game = useGameStatus()
  const [starting, setStarting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  // nothing to start while the game is already running
  if (!game || game.running) return null

  const play = () => {
    setStarting(true)
    setError(null)
    launchGame()
      .then((launched) => {
        // declined in the UAC prompt: nothing happened, nothing to report
        if (!launched) setStarting(false)
        // otherwise the button disappears once the watcher sees the game process
        else setTimeout(() => setStarting(false), 5000)
      })
      .catch((e) => {
        setError(String(e))
        setStarting(false)
      })
  }

  const label = starting ? 'Запуск…' : 'Играть'
  const hint = error ?? (game.path ? undefined : 'Путь к игре не найден. Запустите игру один раз, и Nous его запомнит.')

  return (
    <div className="play">
      <button
        type="button"
        className="play__button"
        onClick={play}
        disabled={starting || !game.path}
        title={game.path ?? undefined}
      >
        <Play size={16} />
        {label}
      </button>
      {hint && <span className="play__hint">{hint}</span>}
    </div>
  )
}
