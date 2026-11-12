// Mirrors world.rs: the stored world and the payload of `world-changed`.

/** 1/1000 m; heading in 1/1000 degree */
export interface Position {
  x: number
  y: number
  z: number
  rotY: number
  layer: number | null
}

export interface World {
  planeId: number | null
  floorId: number | null
  entryId: number | null
  player: Position | null
  /** floor id -> opened MazeChest ids */
  openedChests: Record<string, number[]>
  /** floor id -> chest id -> where the player stood when opening it */
  openedAt: Record<string, Record<string, Position>>
  /** floor id -> "group:config" -> PropState (8 anchor on, 12/13 chest closed/used, …) */
  props: Record<string, Record<string, number>>
}

/** `world-changed`: where the player is and the full data of the changed floors only. */
export interface WorldPatch {
  planeId: number | null
  floorId: number | null
  entryId: number | null
  player: Position | null
  floors: Record<string, { openedChests: number[]; openedAt: Record<string, Position>; props: Record<string, number> }>
}

export function mergeWorld(world: World, patch: WorldPatch): World {
  const next: World = {
    planeId: patch.planeId,
    floorId: patch.floorId,
    entryId: patch.entryId,
    player: patch.player,
    openedChests: { ...world.openedChests },
    openedAt: { ...world.openedAt },
    props: { ...world.props },
  }
  for (const [floor, data] of Object.entries(patch.floors)) {
    next.openedChests[floor] = data.openedChests
    next.openedAt[floor] = data.openedAt
    next.props[floor] = data.props
  }
  return next
}
