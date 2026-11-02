import { useUpdater } from '../../lib/updater'
import { Button } from '../../ui'

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
          <Button size="sm" variant="primary" onClick={install}>
            Обновить
          </Button>
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
