import type { ReactNode } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'

/** Text link to a web page, opened in the browser (not inside Nous). */
export function ExternalLink({ href, children }: { href: string; children: ReactNode }) {
  return (
    <a
      className="ui-link"
      href={href}
      target="_blank"
      rel="noreferrer"
      onClick={(e) => {
        if (!isTauri()) return
        e.preventDefault()
        openUrl(href).catch(console.error)
      }}
    >
      {children}
    </a>
  )
}
