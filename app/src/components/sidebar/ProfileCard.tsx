import type { SyntheticEvent } from 'react'
import { ChevronRight } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { formatNumber, useProfile, type PlayerProfile } from '../../lib/profile'
import { avatarIcons, avatarUrl, DEFAULT_AVATAR } from '../../lib/icons'
import { restartAsAdmin, useSnifferStatus, type SnifferStatus } from '../../lib/sniffer'
import { Button, StatList, StatRow } from '../../ui'

/** Remote picture failed (offline, unknown id): show the bundled default. */
function fallbackAvatar(e: SyntheticEvent<HTMLImageElement>) {
  const img = e.currentTarget
  if (img.src !== avatarIcons[DEFAULT_AVATAR]) img.src = avatarIcons[DEFAULT_AVATAR]
}

/** Why the game login can't be caught right now, if it can't. */
function LoginNotice({ status }: { status: SnifferStatus | null }) {
  if (status?.state === 'needs_admin') {
    return (
      <div className="profile-popover__notice">
        <span className="profile-popover__notice-mark">◆</span>
        <span>
          <b>Нужны права администратора</b>, чтобы Nous увидел вход в игру.
          {/* false = "No" in the UAC prompt: nothing to report */}
          <Button size="sm" onClick={() => restartAsAdmin().catch(console.error)}>
            Перезапустить от администратора
          </Button>
        </span>
      </div>
    )
  }
  if (status?.state === 'error') {
    return (
      <p className="profile-popover__notice">
        <span className="profile-popover__notice-mark">◆</span>
        <span>
          <b>Не удалось следить за игрой:</b> {status.message}
        </span>
      </p>
    )
  }
  return null
}

function ProfilePopover({
  p,
  avatar,
  anchor,
  status,
}: {
  p: PlayerProfile
  avatar: string
  anchor: DOMRect
  status: SnifferStatus | null
}) {
  return (
    <div className="profile-popover acrylic" style={{ top: anchor.top, left: anchor.right + 12 }} role="dialog">
      <div className="profile-popover__head">
        <img className="profile-popover__avatar" src={avatar} alt="" draggable={false} onError={fallbackAvatar} />
        <div className="profile-popover__who">
          <span className="profile-popover__name">{p.nickname ?? 'Первопроходец'}</span>
          <span className="profile-popover__meta">
            {p.uid != null ? `UID ${p.uid}` : 'UID —'}
            {p.level != null && (
              <>
                {' · '}Ур. освоения <span className="profile-popover__level">{p.level}</span>
              </>
            )}
          </span>
        </div>
      </div>

      <StatList>
        <StatRow icon="stellar-jade" label="Звёздный нефрит" value={formatNumber(p.stellarJade)} />
        <StatRow
          icon="credits"
          label="Кредиты"
          value={formatNumber(p.credits, { compact: true })}
          hint={`Кредиты: ${formatNumber(p.credits)}`}
        />
        <StatRow
          icon="trailblaze-power"
          label="Энергия освоения"
          value={formatNumber(p.trailblazePower)}
          max={p.trailblazePowerMax != null ? formatNumber(p.trailblazePowerMax) : undefined}
        />
        <StatRow
          icon="reserved-trailblaze-power"
          label="Резервная энергия освоения"
          value={formatNumber(p.reservedTrailblazePower)}
          max={p.reservedTrailblazePowerMax != null ? formatNumber(p.reservedTrailblazePowerMax) : undefined}
        />
      </StatList>

      {p.source === 'cached' && p.updatedAt != null && (
        <p className="profile-popover__notice">
          <span className="profile-popover__notice-mark">◆</span>
          <span>
            <b>Данные с последнего входа</b> ({new Date(p.updatedAt * 1000).toLocaleString('ru-RU')}). Запустите игру,
            чтобы обновить.
          </span>
        </p>
      )}

      {p.source === 'none' && (
        <p className="profile-popover__notice">
          <span className="profile-popover__notice-mark">◆</span>
          <span>
            <b>Аккаунт не привязан.</b> Запустите игру и войдите в аккаунт, чтобы Nous подтянул ваш профиль.
          </span>
        </p>
      )}

      <LoginNotice status={status} />
    </div>
  )
}

export function ProfileCard() {
  const p = useProfile()
  const status = useSnifferStatus()
  const [anchor, setAnchor] = useState<DOMRect | null>(null)
  const buttonRef = useRef<HTMLButtonElement>(null)
  const open = anchor != null

  useEffect(() => {
    if (!open) return
    const close = (e: Event) => {
      if (e instanceof KeyboardEvent && e.key !== 'Escape') return
      // clicks on the button itself are handled by toggle()
      if (e instanceof MouseEvent && (e.target as Element).closest?.('.profile-popover, .profile')) return
      setAnchor(null)
    }
    const reposition = () => buttonRef.current && setAnchor(buttonRef.current.getBoundingClientRect())
    window.addEventListener('mousedown', close)
    window.addEventListener('keydown', close)
    window.addEventListener('resize', reposition)
    return () => {
      window.removeEventListener('mousedown', close)
      window.removeEventListener('keydown', close)
      window.removeEventListener('resize', reposition)
    }
  }, [open])

  const toggle = () => setAnchor(open ? null : (buttonRef.current?.getBoundingClientRect() ?? null))
  const avatar = avatarUrl(p.avatarId)

  return (
    <>
      <button ref={buttonRef} className={`profile${open ? ' profile--open' : ''}`} onClick={toggle} aria-expanded={open}>
        <div className="profile__avatar">
          <img src={avatar} alt="" draggable={false} onError={fallbackAvatar} />
          {p.level != null && <span className="profile__level">{p.level}</span>}
        </div>
        <div className="profile__who">
          <span className="profile__name">{p.nickname ?? 'Первопроходец'}</span>
          <span className={`profile__status profile__status--${p.source}`}>
            {p.source === 'none' ? 'Аккаунт не привязан' : `UID ${p.uid}`}
          </span>
        </div>
        <ChevronRight size={16} className="profile__chevron" />
      </button>

      {/* portal: the acrylic sidebar (backdrop-filter) would otherwise trap and clip a fixed child */}
      {open && createPortal(<ProfilePopover p={p} avatar={avatar} anchor={anchor} status={status} />, document.body)}
    </>
  )
}
