// Every real modules/*/module.toml must load; bad manifests fail with a clear message.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readdirSync, readFileSync, existsSync } from 'node:fs'
import { loadModule } from '../../src/features/manifest.ts'

const Screen = () => null
const dir = new URL('../../modules/', import.meta.url)

test('all shipped manifests are valid', () => {
  const ids = []
  for (const id of readdirSync(dir)) {
    const path = new URL(`${id}/module.toml`, dir)
    if (!existsSync(path)) continue
    const module = loadModule(`../../modules/${id}/module.toml`, readFileSync(path, 'utf8'), { default: Screen, icon: 'x' })
    assert.equal(module.id, id)
    ids.push(`${id}:${module.kind}`)
  }
  assert.ok(ids.includes('veritas:settings'), 'Veritas is a settings module')
  assert.ok(ids.includes('relics:tab'))
})

const tabToml = `
[module]
id = "t"
title = "T"
kind = "tab"

[tab]
icon = "map"
category = "main"
`

test('a settings module gets a Settings section, a tab a screen', () => {
  const section = loadModule('m/p/module.toml', '[module]\nid = "p"\ntitle = "P"\nkind = "settings"', { default: Screen })
  assert.equal(section.section, Screen)
  assert.equal(section.screens, undefined)
  const tab = loadModule('m/t/module.toml', tabToml, { default: Screen })
  assert.equal(tab.screens[0].component, Screen)
})

test('readable errors for broken manifests', () => {
  const bad = (toml, ui = { default: Screen }) =>
    assert.throws(() => loadModule('m/x/module.toml', toml, ui), /modules\/x\/module\.toml: /)
  const head = '[module]\nid = "x"\ntitle = "X"\n'
  bad('id = "x"\ntitle = "X"\nkind = "tab"')                  // no [module]
  bad('[module]\nid = "y"\ntitle = "X"\nkind = "settings"')     // id differs from folder
  bad(head + 'kind = "widget"')                                // unknown kind
  bad(head + 'kind = "tab"')                                   // tab without [tab]
  bad(head + 'kind = "tab"\n[tab]\nicon = "a"\ncategory = "nowhere"')
  bad(head + 'kind = "tab"\n[tab]\ncategory = "main"')         // no icon
  bad(head + 'kind = "settings"\n[tab]\ncategory = "main"')      // settings with [tab]
  bad(head + 'kind = "settings"', {})                            // no ui
  bad('[module] id = ')                                        // not TOML
})
