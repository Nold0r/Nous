import { useState } from 'react'
import { ShieldAlert } from 'lucide-react'
import { restartAsAdmin, useSnifferStatus } from '../../lib/sniffer'
import { Button } from '../../ui'
import './admin-banner.css'

/**
 * Top of every screen while Nous runs without administrator rights: then it
 * can't see the game at all, so this has to be impossible to miss.
 */
export function AdminBanner() {
  const status = useSnifferStatus()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  if (status?.state !== 'needs_admin') return null

  const restart = () => {
    setBusy(true)
    setError(null)
    restartAsAdmin()
      // false: "No" in the UAC prompt, Nous keeps running as is
      .catch((e) => setError(String(e)))
      .finally(() => setBusy(false))
  }

  return (
    <div className="admin-banner" role="alert">
      <ShieldAlert size={22} className="admin-banner__icon" />
      <div className="admin-banner__text">
        <b>Nous запущен без прав администратора</b>
        <span>{error ?? 'Он не видит игру: профиль, реликвии и бои не обновляются. Перезапустите Nous от имени администратора.'}</span>
      </div>
      <Button variant="primary" size="sm" onClick={restart} disabled={busy}>
        {busy ? 'Ждём подтверждения…' : 'Перезапустить от администратора'}
      </Button>
    </div>
  )
}
