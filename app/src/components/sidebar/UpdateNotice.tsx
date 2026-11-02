import { useUpdater } from '../../lib/updater'
import { Button } from '../../ui'
import { openPatchNotes } from '../../lib/patch-notes'

/** Bottom of the sidebar: offers a newer version from GitHub. */
export function UpdateNotice() {
  const { update, install } = useUpdater()
  if (update.state === 'none') return null

  return (
    <div className="update-notice">
      {update.state === 'available' && (
        <>
          <span className="update-notice__text">
            Доступна версия <b>{update.version}</b>
          </span>
          <span className="update-notice__actions">
            <Button size="sm" variant="ghost" onClick={() => openPatchNotes({ version: update.version, notes: update.notes })}>
              Что нового
            </Button>
            <Button size="sm" variant="primary" onClick={install}>
              Обновить
            </Button>
          </span>
        </>
      )}
      {update.state === 'installing' && (
        <span className="update-notice__text">
          Обновление{update.percent != null ? ` ${update.percent}%` : '…'}
        </span>
      )}
      {update.state === 'error' && (
        <span className="update-notice__text" title={update.message}>
          Не удалось обновить. Попробуйте позже.
        </span>
      )}
    </div>
  )
}
