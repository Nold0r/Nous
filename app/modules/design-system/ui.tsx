import { useState } from 'react'
import { Lock } from 'lucide-react'
import logo from '@app/assets/brand/nous-logo.svg'
import { itemIcons, propertyIcons, signIcons } from '@app/lib/icons'
import {
  Badge,
  Button,
  EmptyState,
  FilterMenu,
  FilterTags,
  GameIcon,
  Grid,
  ItemCard,
  ItemIcon,
  Page,
  Panel,
  PortraitCard,
  ProgressBar,
  Rarity,
  SearchInput,
  SectionTitle,
  Select,
  StatIcon,
  StatLine,
  StatList,
  StatRow,
  Switch,
  Tabs,
  Tag,
  TextArea,
  type FilterState,
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

const sortOptions = [
  { value: 'level', label: 'По уровню' },
  { value: 'new', label: 'Сначала новые' },
  { value: 'set', label: 'По набору' },
] as const

const statOptions = [
  { value: 'crit-rate', label: 'Крит. шанс', icon: <StatIcon icon="IconCriticalChance" size={16} /> },
  { value: 'crit-dmg', label: 'Крит. урон', icon: <StatIcon icon="IconCriticalDamage" size={16} /> },
  { value: 'speed', label: 'Скорость', icon: <StatIcon icon="IconSpeed" size={16} /> },
  { value: 'hp', label: 'НР', icon: <StatIcon icon="IconMaxHP" size={16} /> },
]

const STAR_RAIL_RES = 'https://raw.githubusercontent.com/Mar-7th/StarRailRes/master/'

export function DesignSystemPage() {
  const [tab, setTab] = useState<(typeof tabItems)[number]['id']>('stats')
  const [switchOn, setSwitchOn] = useState(true)
  const [sort, setSort] = useState<(typeof sortOptions)[number]['value']>('level')
  const [stats, setStats] = useState<FilterState>({ 'crit-rate': 'include', hp: 'exclude' })
  const [search, setSearch] = useState('')
  const [note, setNote] = useState('Не разбирать: на Ахерон')

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
            <Button variant="danger">Danger</Button>
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

      <Panel title="Фильтры и поиск" aside="Select · FilterMenu · FilterTags · SearchInput · Tag">
        <div className="ds-row">
          <SearchInput value={search} onChange={setSearch} placeholder="Поиск по заметкам и наборам" />
          <Select title="Порядок" value={sort} options={sortOptions} onChange={setSort} />
          <FilterMenu label="Доп. характеристики" options={statOptions} value={stats} onChange={setStats} />
        </div>
        <FilterTags groups={[{ label: 'Доп.', options: statOptions, value: stats, onChange: setStats }]} onReset={() => setStats({})} />
        <div className="ds-row">
          <Tag>Нейтральный</Tag>
          <Tag tone="include">Показать только</Tag>
          <Tag tone="exclude" onRemove={() => {}}>
            Скрыть
          </Tag>
        </div>
        <span className="text-muted">
          FilterMenu: клик по строке — включить (зелёный, белый список) → исключить (красный, чёрный список) → сбросить; кнопки + и ⊘ задают режим сразу.
        </span>
      </Panel>

      <Grid min={300}>
        <Panel title="Характеристики" aside="StatLine · StatIcon">
          <StatLine icon="IconCriticalDamage" label="Крит. урон" value="64,8%" main showLabel />
          <StatLine icon="IconCriticalChance" label="Крит. шанс" value="9,7%" rolls={2} showLabel />
          <StatLine icon="IconSpeed" label="Скорость" value="7,2" rolls={1} />
          <StatLine icon="IconAttack" label="Сила атаки" value="16" />
          <div className="ds-row">
            {Object.keys(propertyIcons).map((name) => (
              <StatIcon key={name} icon={name} size={22} title={name} />
            ))}
          </div>
        </Panel>

        <Panel title="Текстовое поле" aside="TextArea">
          <TextArea
            label="Заметка"
            value={note}
            onChange={setNote}
            maxLength={2000}
            placeholder="Например: на Ахерон"
            hint="Ctrl+Enter — сохранить"
          />
        </Panel>

        <ItemCard
          image={STAR_RAIL_RES + 'icon/relic/101_0.png'}
          rarity={5}
          title="Карточка предмета (ItemCard)"
          meta="Голова · +15"
          badges={<Lock size={14} />}
          isNew
          onClick={() => {}}
          footer={<span className="text-gold">Подвал: владелец, заметка</span>}
        >
          <StatLine icon="IconMaxHP" label="НР" value="705" main showLabel />
          <StatLine icon="IconCriticalChance" label="Крит. шанс" value="9,7%" rolls={2} />
          <StatLine icon="IconCriticalDamage" label="Крит. урон" value="12,9%" rolls={1} />
        </ItemCard>
      </Grid>

      <SectionTitle aside="PortraitCard">Карточка персонажа</SectionTitle>
      <Grid min={150}>
        <PortraitCard
          image={STAR_RAIL_RES + 'icon/character/1308.png'}
          rarity={5}
          title="Ахерон"
          subtitle="Небытие"
          topRight={<span className="ds-pill">E2</span>}
          bottom={<span>Ур. 80</span>}
          onClick={() => {}}
        />
        <PortraitCard
          image={STAR_RAIL_RES + 'icon/character/1001.png'}
          rarity={4}
          title="Приглушённая (dimmed)"
          subtitle="путь не выбран"
          bottom={<span>Ур. 12</span>}
          dimmed
          onClick={() => {}}
        />
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
