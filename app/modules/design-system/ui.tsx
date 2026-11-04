import { useState } from 'react'
import logo from '@app/assets/brand/nous-logo.svg'
import { itemIcons, signIcons } from '@app/lib/icons'
import {
  Badge,
  Button,
  EmptyState,
  GameIcon,
  Grid,
  ItemIcon,
  Page,
  Panel,
  ProgressBar,
  Rarity,
  SectionTitle,
  StatList,
  StatRow,
  Switch,
  Tabs,
  type Tone,
} from '@app/ui'

const colorTokens = [
  ['--brand-cream', 'Бренд: кремовый'],
  ['--brand-gold', 'Бренд: золото'],
  ['--brand-crimson', 'Бренд: игла'],
  ['--text', 'Основной текст'],
  ['--text-muted', 'Вторичный текст'],
  ['--text-dim', 'Подписи'],
  ['--select-bg', 'Выделение'],
  ['--gold', 'Акцент'],
  ['--success', 'Успех'],
  ['--warning', 'Внимание'],
  ['--danger', 'Ошибка'],
  ['--info', 'Инфо'],
  ['--rarity-5', '5★'],
  ['--rarity-4', '4★'],
  ['--rarity-3', '3★'],
] as const

const typeScale = [
  ['--fs-2xl', 'Заголовок экрана'],
  ['--fs-xl', 'Крупный текст'],
  ['--fs-lg', 'Заголовок блока'],
  ['--fs-md', 'Основной текст'],
  ['--fs-sm', 'Мелкий текст'],
  ['--fs-xs', 'Подписи, бейджи'],
] as const

const tones: Tone[] = ['neutral', 'gold', 'success', 'warning', 'danger', 'info']

const tabItems = [
  { id: 'stats', label: 'Характеристики' },
  { id: 'skills', label: 'Умения' },
] as const

export function DesignSystemPage() {
  const [tab, setTab] = useState<(typeof tabItems)[number]['id']>('stats')
  const [switchOn, setSwitchOn] = useState(true)

  return (
    <Page
      title="Дизайн-система"
      subtitle="Все токены и компоненты Nous вживую. Документация: docs/Дизайн-система"
      actions={
        <>
          <Button variant="ghost">Второстепенное</Button>
          <Button variant="primary">Главное действие</Button>
        </>
      }
    >
      <Panel title="Логотип" aside="assets/brand/nous-logo.svg">
        <div className="ds-row">
          {[160, 64, 38, 24].map((size) => (
            <img key={size} src={logo} width={size} height={size} alt="" />
          ))}
          <span className="text-muted">
            Игла (малиновый) — фирменный акцент: тонкие линии и точки, например линия под заголовком экрана.
          </span>
        </div>
      </Panel>

      <Panel title="Цвета" aside="tokens.css">
        <Grid min={150}>
          {colorTokens.map(([token, name]) => (
            <div key={token} className="ds-swatch">
              <span className="ds-swatch__color" style={{ background: `var(${token})` }} />
              <span>{name}</span>
              <code className="text-dim">{token}</code>
            </div>
          ))}
        </Grid>
      </Panel>

      <Panel title="Типографика" aside="шрифт HSR (RPG_CN)">
        {typeScale.map(([token, name]) => (
          <div key={token} className="ds-type">
            <span style={{ fontSize: `var(${token})` }}>{name} · Звёздный экспресс 1234</span>
            <code className="text-dim">{token}</code>
          </div>
        ))}
      </Panel>

      <Grid min={360}>
        <Panel title="Кнопки">
          <div className="ds-row">
            <Button variant="primary">Primary</Button>
            <Button>Secondary</Button>
            <Button variant="ghost">Ghost</Button>
            <Button disabled>Disabled</Button>
          </div>
          <div className="ds-row">
            <Button variant="primary" size="sm" icon="map">
              С иконкой
            </Button>
            <Button size="sm">Подробнее</Button>
          </div>
        </Panel>

        <Panel title="Переключатель">
          <Switch checked={switchOn} onChange={setSwitchOn} label="Настройка" hint="Пояснение под названием" />
          <Switch checked={false} onChange={() => {}} disabled label="Недоступно" />
        </Panel>

        <Panel title="Вкладки и метки">
          <Tabs items={tabItems} value={tab} onChange={setTab} />
          <div className="ds-row">
            {tones.map((t) => (
              <Badge key={t} tone={t}>
                {t}
              </Badge>
            ))}
          </div>
          <div className="ds-row">
            <Rarity stars={5} />
            <Rarity stars={4} />
            <Rarity stars={3} />
          </div>
        </Panel>

        <Panel title="Строки характеристик">
          <StatList>
            <StatRow icon="stellar-jade" label="Звёздный нефрит" value="12 480" />
            <StatRow icon="trailblaze-power" label="Энергия освоения" value="184" max="300" />
            <StatRow icon={<GameIcon icon="characters" size={20} />} label="С игровой иконкой" value="61,9%" />
          </StatList>
        </Panel>

        <Panel title="Прогресс">
          <ProgressBar label="Исследование" value={64} max={100} />
          <ProgressBar label="Сундуки" value={8} max={10} tone="gold" />
          <ProgressBar label="Готово" value={3} max={3} tone="success" />
        </Panel>
      </Grid>

      <SectionTitle aside="Panel variant / compact / onClick">Панели</SectionTitle>
      <Grid min={220}>
        <Panel compact>Акриловая (по умолчанию)</Panel>
        <Panel compact variant="plain">
          Plain: для вложенных панелей
        </Panel>
        <Panel compact onClick={() => {}}>
          Кликабельная: наведите
        </Panel>
      </Grid>

      <Panel title="Иконки" aside="npm run assets">
        <div className="ds-row">
          {Object.keys(signIcons).map((name) => (
            <span key={name} className="ds-icon" title={name}>
              <GameIcon icon={name} size={28} />
              <code>{name}</code>
            </span>
          ))}
        </div>
        <div className="ds-row">
          {Object.keys(itemIcons).map((name) => (
            <span key={name} className="ds-icon" title={name}>
              <ItemIcon name={name} size={36} />
              <code>{name}</code>
            </span>
          ))}
        </div>
      </Panel>

      <EmptyState icon="inventory" title="Пустое состояние" action={<Button size="sm">Действие</Button>}>
        Так выглядит экран или блок, когда данных ещё нет.
      </EmptyState>
    </Page>
  )
}

export default DesignSystemPage
export { Palette as icon } from 'lucide-react'
