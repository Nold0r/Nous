import { Fragment, type ReactNode } from 'react'
import { openUrl } from '@tauri-apps/plugin-opener'

// The small Markdown subset patch notes use: paragraphs, "- " lists,
// **bold**, `code`, [links](https://…). Built as React elements, no innerHTML.

function inline(text: string): ReactNode[] {
  const out: ReactNode[] = []
  const re = /\*\*(.+?)\*\*|`([^`]+)`|\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)/g
  let last = 0
  for (let m = re.exec(text); m; m = re.exec(text)) {
    if (m.index > last) out.push(text.slice(last, m.index))
    const key = out.length
    if (m[1] !== undefined) out.push(<b key={key}>{inline(m[1])}</b>)
    else if (m[2] !== undefined) out.push(<code key={key}>{m[2]}</code>)
    else {
      const url = m[4]
      out.push(
        <a
          key={key}
          href={url}
          onClick={(e) => {
            e.preventDefault()
            openUrl(url).catch(console.error)
          }}
        >
          {m[3]}
        </a>,
      )
    }
    last = re.lastIndex
  }
  if (last < text.length) out.push(text.slice(last))
  return out
}

export function Markdown({ text }: { text: string }) {
  const blocks: ReactNode[] = []
  let list: string[] = []
  let para: string[] = []
  const flush = () => {
    if (para.length) blocks.push(<p key={blocks.length}>{inline(para.join(' '))}</p>)
    if (list.length)
      blocks.push(
        <ul key={blocks.length}>
          {list.map((item, i) => (
            <li key={i}>{inline(item)}</li>
          ))}
        </ul>,
      )
    para = []
    list = []
  }
  for (const line of text.replace(/\r\n/g, '\n').split('\n')) {
    const t = line.trim()
    if (!t || t === '---') {
      flush()
    } else if (t.startsWith('- ')) {
      if (para.length) flush()
      list.push(t.slice(2))
    } else if (list.length && line.startsWith('  ')) {
      list[list.length - 1] += ' ' + t // continuation of a list item
    } else {
      if (list.length) flush()
      para.push(t)
    }
  }
  flush()
  return <Fragment>{blocks}</Fragment>
}
