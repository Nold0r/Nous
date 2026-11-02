// Opens the patch notes dialog (components/patch-notes) from anywhere.

/** A version that's offered but not installed yet (from the update check). */
export interface Upcoming {
  version: string
  notes: string
}

export interface PatchNotesState {
  open: boolean
  upcoming: Upcoming | null
}

let state: PatchNotesState = { open: false, upcoming: null }
const listeners = new Set<() => void>()

export function setPatchNotes(next: PatchNotesState) {
  state = next
  listeners.forEach((l) => l())
}

export function subscribePatchNotes(l: () => void) {
  listeners.add(l)
  return () => {
    listeners.delete(l)
  }
}

export function patchNotesState(): PatchNotesState {
  return state
}

/** Opens the patch notes; with `upcoming`, the offered update is shown on top. */
export function openPatchNotes(upcoming: Upcoming | null = null) {
  setPatchNotes({ open: true, upcoming })
}
