import { useState } from 'react'
import { EmptyState, Page, Tabs } from '@app/ui'
import { useWorld } from '@app/lib/game-data'
import { floorLabel, floorsLowToHigh, mapForFloor, playerLayer } from './maps'
import { MapView } from './MapView'
import './map.css'

/**
 * Map of the location the player is in, nothing else: the floor drawn like
 * the game's minimap (from the game's own textures) and the player's arrow.
 */
export function MapPage() {
  const world = useWorld()
  const map = mapForFloor(world?.floorId)
  const player = world?.player ?? null
  const auto = map ? playerLayer(map, player) : null
  // a floor picked by hand holds until the location or the game's floor changes
  const scope = `${world?.floorId}:${auto?.layer}`
  const [picked, setPicked] = useState<{ scope: string; id: number } | null>(null)

  if (!world?.floorId) {
    return (
      <Page title="Карта" subtitle="Текущая локация">
        <EmptyState icon="map" title="Локация неизвестна">
          Войдите в игру с запущенным Nous: карта покажет локацию, где вы находитесь.
        </EmptyState>
      </Page>
    )
  }
  if (!map || !auto) {
    return (
      <Page title="Карта" subtitle="Текущая локация">
        <EmptyState icon="map" title="Для этой локации карты нет">
          Карты есть для локаций с миникартой в игре. В Виртуальной вселенной и в событиях карты нет.
        </EmptyState>
      </Page>
    )
  }

  const layer = (picked?.scope === scope && map.layers.find((l) => l.layer === picked.id)) || auto
  const floors = floorsLowToHigh(map.layers).reverse()
  return (
    <Page
      title="Карта"
      subtitle={map.name}
      actions={
        floors.length > 1 && (
          <Tabs
            items={floors.map((f) => ({ id: String(f.layer), label: floorLabel(f.name) + (f.layer === auto.layer ? ' •' : '') }))}
            value={String(layer.layer)}
            onChange={(id) => setPicked({ scope, id: Number(id) })}
          />
        )
      }
    >
      <MapView map={map} layer={layer} player={player} />
      <p className="map-hint">Колесо — масштаб, перетаскивание — сдвиг, двойной клик — вся карта</p>
    </Page>
  )
}

export default MapPage
