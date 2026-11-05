import manifest from './manifest.json'

// Animated splash arts (Spine skeletons) of the characters, as prepared by
// the HSR Optimizer project (github.com/fribbels/hsr-optimizer, MIT): the
// game's own animations, extracted. manifest.json: base character id ->
// number of skeletons drawn on top of each other. Files are loaded from that
// repository like the StarRailRes pictures.
// Docs: docs/Архитектура/Модуль персонажей.md

const SPINE_BASE = 'https://raw.githubusercontent.com/fribbels/hsr-optimizer/main/public/assets/spine/'

/** "1308", also for path ids like 8010 (first four digits, as the source does). */
const baseId = (characterId: number | string) => String(characterId).slice(0, 4)

/** Skeleton files of a character, or null when it has no animated art. */
export function spineFiles(characterId: number | string): { baseUrl: string; files: { skel: string; atlas: string }[] } | null {
  const id = baseId(characterId)
  const count = (manifest as Record<string, number>)[id]
  if (!count) return null
  const files =
    count === 1
      ? [{ skel: `${id}.skel`, atlas: `${id}.atlas` }]
      : Array.from({ length: count }, (_, i) => ({ skel: `${id}_${i}.skel`, atlas: `${id}_${i}.atlas` }))
  return { baseUrl: `${SPINE_BASE}${id}/`, files }
}
