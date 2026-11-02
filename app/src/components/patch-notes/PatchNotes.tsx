import { useEffect, useState, useSyncExternalStore } from 'react'
import { getVersion } from '@tauri-apps/api/app'
import { isTauri } from '@tauri-apps/api/core'
import { changelog, stripFooter } from '../../lib/changelog'
import { openPatchNotes, patchNotesState, setPatchNotes, subscribePatchNotes } from '../../lib/patch-notes'
import { Badge, Button, Modal } from '../../ui'
import { Markdown } from './Markdown'
import './patch-notes.css'

const SEEN_KEY = 'nous:patch-notes:seen-version'

function readSeen(): string | null {
  try {
    return localStorage.getItem(SEEN_KEY)
  } catch {
    return null
  }
}

function writeSeen(version: string) {
  try {
    localStorage.setItem(SEEN_KEY, version)
  } catch {
    // private storage unavailable: the dialog may show again, harmless
  }
}

/**
 * Mounted once in the shell. Shows the patch notes when asked, and on its own
 * at the first start of each version that has notes.
 */
export function PatchNotesHost() {
  const { open, upcoming } = useSyncExternalStore(subscribePatchNotes, patchNotesState)
  const [current, setCurrent] = useState<string | null>(null)

  useEffect(() => {
    if (!isTauri()) return
    getVersion()
      .then((v) => {
        setCurrent(v)
        // first start of this version (also right after updating from a build
        // that didn't remember anything yet): show what's new once
        if (readSeen() !== v && changelog(v).some((e) => e.version === v)) openPatchNotes()
      })
      .catch(console.error)
  }, [])

  const close = () => {
    if (current) writeSeen(current)
    setPatchNotes({ open: false, upcoming: null })
  }

  const entries = changelog(current)
  return (
    <Modal
      open={open}
      title="Что нового в Nous"
      onClose={close}
      footer={
        <Button variant="primary" onClick={close}>
          Понятно
        </Button>
      }
    >
      <div className="patch-notes">
        {upcoming && (
          <section className="patch-notes__entry">
            <header className="patch-notes__head">
              <span className="patch-notes__version">Версия {upcoming.version}</span>
              <Badge tone="info">Доступно обновление</Badge>
            </header>
            <Markdown text={stripFooter(upcoming.notes) || 'Описание не указано.'} />
          </section>
        )}
        {entries.map((e, i) => (
          <section key={i} className="patch-notes__entry">
            <header className="patch-notes__head">
              <span className="patch-notes__version">{e.version ? `Версия ${e.version}` : 'Следующая версия'}</span>
              {e.date && <span className="patch-notes__date">{new Date(e.date).toLocaleDateString('ru-RU')}</span>}
              {e.version && e.version === current && <Badge tone="success">Установлена</Badge>}
            </header>
            <Markdown text={e.body} />
          </section>
        ))}
        {entries.length === 0 && !upcoming && <p className="patch-notes__empty">Список изменений пуст.</p>}
      </div>
    </Modal>
  )
}
