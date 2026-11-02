import { useEffect, useState } from 'react'
import { avatarIcons } from '../lib/icons'

/** Anything Veritas names: a character or an enemy. */
type Named = { id: number; name: string }

// Portraits come from StarRailRes like the rest of Nous; enemy pictures from
// the Hakushin/nanoka mirror, the only public one with monster art.
const STAR_RAIL_RES = 'https://raw.githubusercontent.com/Mar-7th/StarRailRes/master/'
const NANOKA = 'https://static.nanoka.cc'

type Indexes = { characters: Map<string, string>; monsters: Map<number, string> }
let loading: Promise<Indexes> | null = null

/** Russian character name -> id (trial versions of characters have their own ids). */
async function characterIndex(): Promise<Map<string, string>> {
  const data: Record<string, { id: string; name: string }> = await (await fetch(`${STAR_RAIL_RES}index_new/ru/characters.json`)).json()
  return new Map(Object.values(data).map((c) => [c.name.toLowerCase(), c.id]))
}

/** Monster instance id (as Veritas reports it) -> picture file stem. */
async function monsterIndex(): Promise<Map<number, string>> {
  const manifest = await (await fetch(`${NANOKA}/manifest.json`)).json()
  const data: Record<string, { icon: string; child?: number[] }> = await (await fetch(`${NANOKA}/hsr/${manifest.hsr.live}/monster.json`)).json()
  const out = new Map<number, string>()
  for (const [id, monster] of Object.entries(data)) {
    const stem = monster.icon.split('/').pop()!.replace(/\.png$/, '')
    out.set(Number(id), stem)
    for (const child of monster.child ?? []) out.set(child, stem)
  }
  return out
}

function load(): Promise<Indexes> {
  loading ??= Promise.all([
    characterIndex().catch(() => new Map<string, string>()),
    monsterIndex().catch(() => new Map<number, string>()),
  ]).then(([characters, monsters]) => ({ characters, monsters }))
  return loading
}

function characterUrl(avatar: Named, characters: Map<string, string>): string | undefined {
  // playable characters are 1xxx, the Trailblazer 8xxx; others are trial copies
  const real = avatar.id < 7000 || avatar.id >= 8000 ? String(avatar.id) : characters.get(avatar.name.toLowerCase())
  if (!real) return undefined
  return avatarIcons[real] ?? `${STAR_RAIL_RES}icon/avatar/${real}.png`
}

/** Picture lookups for a battle; re-renders once the indexes arrive. */
export function useBattleIcons() {
  const [indexes, setIndexes] = useState<Indexes | null>(null)
  useEffect(() => {
    let alive = true
    load().then((i) => { if (alive) setIndexes(i) })
    return () => { alive = false }
  }, [])
  return {
    character: (avatar: Named) => characterUrl(avatar, indexes?.characters ?? new Map()),
    enemy: (enemy: Named) => {
      const stem = indexes?.monsters.get(enemy.id)
      return stem ? `${NANOKA}/assets/hsr/monstermiddleicon/${stem}.webp` : undefined
    },
  }
}
