import manifest from './manifest.json'
import centers from './centers.json'

// Animated splash arts (Spine skeletons) of the characters, as prepared by
// the HSR Optimizer project (github.com/fribbels/hsr-optimizer, MIT): the
// game's own animations, extracted. manifest.json: base character id ->
// number of skeletons drawn on top of each other. Files are loaded from that
// repository like the StarRailRes pictures.
// Docs: docs/Архитектура/Модуль персонажей.md

const SPINE_BASE = 'https://raw.githubusercontent.com/fribbels/hsr-optimizer/main/public/assets/spine/'

/** "1308", also for path ids like 8010 (first four digits, as the source does). */
const baseId = (characterId: number | string) => String(characterId).slice(0, 4)

/** A point of the 2048 px art to put in the middle of the frame, and the zoom. */
export interface ArtCenter {
  x: number
  y: number
  z: number
}

const DEFAULT_CENTER: ArtCenter = { x: 1024, y: 1024, z: 1 }
type Framing = { image: ArtCenter; spine: ArtCenter; disableSpine: boolean }
const framing = (characterId: number | string): Framing | undefined =>
  (centers as Record<string, Framing>)[String(characterId)] ?? (centers as Record<string, Framing>)[baseId(characterId)]

/**
 * Framing of a character's art (centers.json, tuned by hand in HSR
 * Optimizer, `npm run spine-centers`): `image` for the still portrait,
 * `spine` for the animation.
 */
export function artCenters(characterId: number | string): { image: ArtCenter; spine: ArtCenter } {
  const f = framing(characterId)
  return { image: f?.image ?? DEFAULT_CENTER, spine: f?.spine ?? f?.image ?? DEFAULT_CENTER }
}

/**
 * Skeleton files of a character, or null when it has no animated art or
 * the optimizer turned its animation off (it looks wrong) — then the still
 * portrait is shown, as there.
 */
export function spineFiles(characterId: number | string): { baseUrl: string; files: { skel: string; atlas: string }[] } | null {
  const id = baseId(characterId)
  const count = (manifest as Record<string, number>)[id]
  if (!count || framing(characterId)?.disableSpine) return null
  const files =
    count === 1
      ? [{ skel: `${id}.skel`, atlas: `${id}.atlas` }]
      : Array.from({ length: count }, (_, i) => ({ skel: `${id}_${i}.skel`, atlas: `${id}_${i}.atlas` }))
  return { baseUrl: `${SPINE_BASE}${id}/`, files }
}
