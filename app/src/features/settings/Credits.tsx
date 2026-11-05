import { useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { Button, Modal, Panel } from '../../ui'

// Who owns what Nous shows, and the licenses of what it is built with.
// HoYoverse's fan content guide asks fan works to carry the copyright
// notice; the Spine runtime and HSR Optimizer code require their license
// texts to ship with the app (public/licenses).
// Docs: docs/Архитектура/Модуль персонажей.md

const open = (url: string) => (isTauri() ? openUrl(url).catch(console.error) : window.open(url, '_blank'))

const CREDITS: { title: string; text: string; url?: string; license?: string }[] = [
  {
    title: 'Honkai: Star Rail',
    text: 'Игра, персонажи, арты, иконки, шрифт и другие игровые материалы © HoYoverse. All rights reserved by miHoYo. Используются по правилам фан-творчества Honkai: Star Rail для некоммерческой фанатской программы.',
    url: 'https://hsr.hoyoverse.com/en-us/news/111203?type=news',
  },
  {
    title: 'StarRailRes',
    text: 'Таблицы игры (названия, характеристики, следы, эйдолоны) и картинки персонажей, реликвий, конусов и предметов.',
    url: 'https://github.com/Mar-7th/StarRailRes',
  },
  {
    title: 'HSR Optimizer (Fribbels)',
    text: 'Анимированные арты персонажей и их кадрирование; движок анимации Nous основан на их коде. Лицензия MIT.',
    url: 'https://github.com/fribbels/hsr-optimizer',
    license: 'hsr-optimizer.txt',
  },
  {
    title: 'Spine Runtimes',
    text: 'Проигрывание анимированных артов. © Esoteric Software, Spine Runtimes License.',
    url: 'http://esotericsoftware.com',
    license: 'spine-runtimes.txt',
  },
  {
    title: 'reliquary',
    text: 'Разбор сетевого протокола игры для чтения данных аккаунта.',
    url: 'https://github.com/IceDynamix/reliquary',
  },
]

/** "Правообладатели" in Settings: the notices and licenses. */
export function Credits() {
  const [license, setLicense] = useState<{ title: string; text: string } | null>(null)

  const showLicense = (title: string, file: string) =>
    fetch(`licenses/${file}`)
      .then((r) => r.text())
      .then((text) => setLicense({ title, text }))
      .catch(console.error)

  return (
    <Panel title="Правообладатели">
      <p className="credits-lead">
        Nous — фанатская программа. Она не связана с HoYoverse, не одобрена и не поддерживается ею.
      </p>
      <div className="credits">
        {CREDITS.map((c) => (
          <div key={c.title} className="credits__item">
            <div className="credits__text">
              <span className="credits__title">{c.title}</span>
              <span className="credits__desc">{c.text}</span>
            </div>
            <div className="credits__actions">
              {c.license && (
                <Button size="sm" variant="ghost" onClick={() => showLicense(c.title, c.license!)}>
                  Лицензия
                </Button>
              )}
              {c.url && (
                <Button size="sm" variant="ghost" onClick={() => open(c.url!)}>
                  Сайт
                </Button>
              )}
            </div>
          </div>
        ))}
      </div>
      <Modal open={!!license} title={license ? `${license.title}: лицензия` : ''} onClose={() => setLicense(null)}>
        <pre className="credits-license">{license?.text}</pre>
      </Modal>
    </Panel>
  )
}
