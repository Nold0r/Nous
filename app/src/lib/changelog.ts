import raw from '../../../CHANGELOG.md?raw'

// Patch notes bundled into the app from CHANGELOG.md at the repo root.
// CI builds a release from the "## Unreleased" section, so inside a build that
// section is the notes of the running version.

export interface ChangelogEntry {
  /** "0.1.8", or null for an Unreleased section with unknown version */
  version: string | null
  date: string | null
  /** Markdown of the section */
  body: string
}

/** All sections, newest first; Unreleased gets `currentVersion`. Empty sections are skipped. */
export function changelog(currentVersion: string | null): ChangelogEntry[] {
  const entries: ChangelogEntry[] = []
  const sections = raw.replace(/\r\n/g, '\n').split(/^## /m).slice(1)
  for (const section of sections) {
    const nl = section.indexOf('\n')
    const head = (nl < 0 ? section : section.slice(0, nl)).trim()
    const body = (nl < 0 ? '' : section.slice(nl + 1)).trim()
    if (!body) continue
    if (head === 'Unreleased') {
      // a dev build's Unreleased is the upcoming version, not a shipped one
      entries.push({ version: currentVersion, date: null, body })
      continue
    }
    const m = head.match(/^v(\d+\.\d+\.\d+)\s*(?:—\s*(\S+))?/)
    if (m) entries.push({ version: m[1], date: m[2] ?? null, body })
  }
  return entries
}

/** Notes text of a published release (latest.json / GitHub), without the install footer. */
export function stripFooter(notes: string): string {
  return notes.replace(/\r\n/g, '\n').split(/\n---\n/)[0].trim()
}
