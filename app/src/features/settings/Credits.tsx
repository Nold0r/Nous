import { useState } from 'react'
import { ExternalLink, Modal, Panel } from '../../ui'

// Who owns what Nous shows, and the licenses of what it is built with.
// HoYoverse's fan content guide asks fan works to carry the copyright
// notice; the Spine runtime and HSR Optimizer code require their license
// texts to ship with the app (public/licenses).
// Docs: docs/Архитектура/Модуль персонажей.md

const CREDITS: { title: string; url: string; text: string; license?: string }[] = [
  {
    title: 'Honkai: Star Rail',
    url: 'https://hsr.hoyoverse.com/en-us/news/111203?type=news',
    text: 'игра, арты, иконки, шрифт © HoYoverse. All rights reserved by miHoYo',
  },
  { title: 'StarRailRes', url: 'https://github.com/Mar-7th/StarRailRes', text: 'таблицы игры и картинки' },
  {
    title: 'HSR Optimizer',
    url: 'https://github.com/fribbels/hsr-optimizer',
    text: 'анимированные арты и их кадр, MIT',
    license: 'hsr-optimizer.txt',
  },
  {
    title: 'Spine Runtimes',
    url: 'http://esotericsoftware.com',
    text: 'проигрывание анимаций © Esoteric Software',
    license: 'spine-runtimes.txt',
  },
  { title: 'reliquary', url: 'https://github.com/IceDynamix/reliquary', text: 'разбор протокола игры' },
]

/** "Правообладатели" in Settings: the notices and licenses, one line each. */
export function Credits() {
  const [license, setLicense] = useState<{ title: string; text: string } | null>(null)

  const showLicense = (title: string, file: string) =>
    fetch(`licenses/${file}`)
      .then((r) => r.text())
      .then((text) => setLicense({ title, text }))
      .catch(console.error)

  return (
    <Panel title="Правообладатели">
      <p className="credits-lead">Nous — фанатская программа, не связана с HoYoverse и не одобрена ею.</p>
      <ul className="credits">
        {CREDITS.map((c) => (
          <li key={c.title}>
            <ExternalLink href={c.url}>{c.title}</ExternalLink>
            <span className="credits__desc"> — {c.text}</span>
            {c.license && (
              <button type="button" className="credits__license" onClick={() => showLicense(c.title, c.license!)}>
                лицензия
              </button>
            )}
          </li>
        ))}
      </ul>
      <Modal open={!!license} title={license ? `${license.title}: лицензия` : ''} onClose={() => setLicense(null)}>
        <pre className="credits-license">{license?.text}</pre>
      </Modal>
    </Panel>
  )
}
