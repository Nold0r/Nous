---
tags: [nous, sdk, архитектура, api, проект]
status: draft
date: 2026-11-03
---

# Nous SDK: проект объектного API

Приблизительный публичный контракт встраиваемой библиотеки для получения информации из Honkai: Star Rail. Это проект структуры, а не описание уже реализованных методов. Исходный код SDK этим документом не изменяется.

Основа — существующие `nous-core`, `nous-hsr`, `nous-veritas` в соседнем checkout `Nous-sdk`. Дополнительные категории выделены по локальным схемам протокола; наличие схемы не доказывает доступность данных в живой игре.

## 1. Требования

- Библиотека встраивается в стороннее приложение и работает без окна Nous, Tauri и HTTP-сервера.
- Данные аккуратно упакованы в предметные модели, строго типизированы и унифицированы там, где смысл совпадает.
- Профиль, кредиты и звёздный нефрит — отдельные сервисы. Пространственная модель включает этажи, слои, комнаты и их типы, экземпляры, сцены и состояние окружения.
- Единый вход `NousSdk`; категории представлены объектами-сервисами.
- Чтение состояния не отправляет игровые пакеты и не инициирует скрытые сетевые операции.
- Кеш, наблюдения текущего подключения, внешние импорты и вычисленные показатели различимы.
- Нельзя обещать «всю информацию»: библиотека предоставляет известные данные от подключённых источников и явно описывает пробелы.

Выбранный для этого наброска подход — композиция предметных сервисов с общими контрактами чтения и событий. Это предложение автора документа, а не согласованное решение о реализации. Единый `get(category, key) -> JSON` хуже сохраняет типы; глубокая иерархия наследования затрудняет расширение. В Rust роли классов выполняют `struct`, `impl` и небольшие `trait`.

## 2. Объекты верхнего уровня

```text
NousSdk                                      фасад библиотеки
├── runtime: RuntimeService                  запуск/остановка источников
├── capabilities: CapabilityService          поддержка данных и методов
├── sessions: SessionService                 подтверждённые подключения
├── accounts: AccountRepository              известные аккаунты и их кеши
├── catalog: GameCatalog                     справочники ресурсов игры
├── imports: ImportService                   явно запускаемые импорты
├── diagnostics: DiagnosticsService          состояние и ошибки источников
└── observe(...): Subscription<SdkEvent>      общие события

sdk.accounts.open(accountId) -> AccountApi    объект закреплён за UID
sdk.accounts.current() -> Option<AccountApi> текущий подтверждённый UID
sdk.sessions.current() -> Option<SessionApi> текущее подтверждённое подключение
```

`AccountApi` никогда не меняет UID вслед за активным аккаунтом. При переключении A → B ранее полученный объект A продолжает читать A. До подтверждения нового подключения `accounts.current()` не выдаёт последний сохранённый UID за текущего игрока. Для последнего кеша есть отдельный `accounts.last_observed()`.

## 3. Категории данных аккаунта

Все сигнатуры ниже условные. Короткая запись возвращаемого типа означает:

```text
Read<T>       = Result<DataState<T>, SdkError>
ReadList<T>   = Read<Page<T>>
Watch<T>      = Result<Subscription<Change<T>>, SdkError>
```

`list(query)` получает типизированный фильтр с пагинацией. `get(id)` читает одну сущность. `snapshot()` читает весь небольшой раздел согласованно. `observe(filter)` подписывается на изменения. Конкретные сервисы предоставляют только осмысленные для них операции: общего обязательного CRUD нет.

```text
AccountApi
├── profile: ProfileService
│   ├── get() -> Read<PlayerProfile>
│   └── observe() -> Watch<PlayerProfile>
│
├── credits: CreditsService
│   ├── balance() -> Read<CurrencyBalance<Credits>>
│   ├── history(CurrencyHistoryQuery) -> ReadList<CurrencyChange<Credits>>
│   └── observe() -> Watch<CurrencyChange<Credits>>
│
├── stellar_jade: StellarJadeService
│   ├── balance() -> Read<CurrencyBalance<StellarJade>>
│   ├── history(CurrencyHistoryQuery) -> ReadList<CurrencyChange<StellarJade>>
│   └── observe() -> Watch<CurrencyChange<StellarJade>>
│
├── currencies: CurrencyService
│   ├── list(CurrencyQuery) -> ReadList<CurrencyBalanceEntry>
│   ├── balance(CurrencyId) -> Read<CurrencyBalanceEntry>
│   └── observe(CurrencyFilter) -> Watch<CurrencyBalanceChange>
│
├── energy: EnergyService
│   ├── get() -> Read<EnergyState>
│   ├── forecast(at: Timestamp) -> Read<EnergyForecast>
│   └── observe() -> Watch<EnergyState>
│
├── characters: CharacterService
│   ├── list(CharacterQuery) -> ReadList<Character>
│   ├── get(CharacterId) -> Read<Character>
│   ├── progression(CharacterId) -> Read<CharacterProgression>
│   ├── equipment(CharacterId) -> Read<CharacterEquipment>
│   └── observe(CharacterFilter) -> Watch<CharacterChange>
│
├── inventory: InventoryApi
│   ├── relics: RelicService
│   │   ├── list(RelicQuery) -> ReadList<Relic>
│   │   ├── get(RelicUid) -> Read<Relic>
│   │   ├── history(RelicHistoryQuery) -> ReadList<RelicChange>
│   │   └── observe(RelicFilter) -> Watch<RelicChange>
│   ├── light_cones: LightConeService
│   │   ├── list(LightConeQuery) -> ReadList<LightCone>
│   │   ├── get(LightConeUid) -> Read<LightCone>
│   │   └── observe(LightConeFilter) -> Watch<LightConeChange>
│   ├── materials: MaterialService
│   │   ├── list(MaterialQuery) -> ReadList<MaterialStack>
│   │   └── get(ItemId) -> Read<MaterialBalance>
│   └── consumables: ConsumableService
│       └── list(ConsumableQuery) -> ReadList<ConsumableStack>
│
├── teams: TeamService
│   ├── list(TeamQuery) -> ReadList<Team>
│   ├── active() -> Read<Team>
│   └── observe() -> Watch<TeamChange>
│
├── world: WorldApi
│   ├── context() -> Read<WorldContext>
│   ├── snapshot(WorldSectionSet) -> Read<WorldSnapshot>
│   ├── worlds: WorldService
│   │   ├── list(WorldQuery) -> ReadList<WorldProgress>
│   │   └── get(WorldId) -> Read<WorldProgress>
│   ├── planes: PlaneService
│   │   ├── list(PlaneQuery) -> ReadList<PlaneProgress>
│   │   └── get(PlaneId) -> Read<PlaneProgress>
│   ├── floors: FloorService
│   │   ├── list(FloorQuery) -> ReadList<FloorState>
│   │   ├── get(WorldSpaceKey) -> Read<FloorState>
│   │   └── layers(WorldSpaceKey) -> ReadList<MapLayerState>
│   ├── areas: AreaService
│   │   └── list(AreaQuery) -> ReadList<AreaState>
│   ├── rooms: RoomService
│   │   ├── current() -> Read<RoomState>
│   │   ├── list(RoomQuery) -> ReadList<RoomState>
│   │   ├── get(RoomKey) -> Read<RoomState>
│   │   ├── exits(RoomKey) -> ReadList<RoomExit>
│   │   └── observe(RoomFilter) -> Watch<RoomChange>
│   ├── instances: WorldInstanceService
│   │   ├── list(InstanceQuery) -> ReadList<WorldInstanceState>
│   │   └── get(InstanceKey) -> Read<WorldInstanceState>
│   ├── exploration: ExplorationService
│   │   ├── get(WorldSpaceKey) -> Read<ExplorationProgress>
│   │   └── sections(WorldSpaceKey) -> ReadList<MapSectionProgress>
│   ├── teleports: TeleportService
│   │   ├── list(TeleportQuery) -> ReadList<TeleportState>
│   │   └── get(TeleportKey) -> Read<TeleportState>
│   ├── chests: ChestService
│   │   ├── list(ChestQuery) -> ReadList<ChestObservation>
│   │   └── get(ChestKey) -> Read<ChestObservation>
│   ├── objects: WorldObjectService
│   │   ├── list(WorldObjectQuery) -> ReadList<WorldObjectObservation>
│   │   └── get(WorldObjectKey) -> Read<WorldObjectObservation>
│   ├── groups: WorldGroupService
│   │   └── list(WorldGroupQuery) -> ReadList<WorldGroupState>
│   ├── mechanisms: MechanismService
│   │   ├── list(MechanismQuery) -> ReadList<MechanismState>
│   │   └── get(WorldObjectKey) -> Read<MechanismState>
│   ├── state: WorldStateService
│   │   ├── flags(WorldFlagQuery) -> ReadList<WorldFlagState>
│   │   └── missions(WorldSpaceKey) -> Read<SceneMissionProgress>
│   └── routes: RouteHistoryService
│       └── list(RouteQuery) -> ReadList<RouteSegment>
│
├── missions: MissionService
│   ├── list(MissionQuery) -> ReadList<MissionProgress>
│   ├── get(MissionId) -> Read<MissionProgress>
│   └── observe(MissionFilter) -> Watch<MissionChange>
│
├── achievements: AchievementService
│   ├── list(AchievementQuery) -> ReadList<AchievementProgress>
│   ├── get(AchievementId) -> Read<AchievementProgress>
│   └── summary() -> Read<AchievementSummary>
│
├── activities: ActivityApi
│   ├── daily: DailyActivityService
│   │   └── get() -> Read<DailyActivityProgress>
│   ├── assignments: AssignmentService
│   │   └── list(AssignmentQuery) -> ReadList<AssignmentProgress>
│   ├── battle_pass: BattlePassService
│   │   └── get() -> Read<BattlePassProgress>
│   └── events: EventProgressService
│       ├── list(EventQuery) -> ReadList<EventProgress>
│       └── get(ActivityId) -> Read<EventProgress>
│
├── challenges: ChallengeService
│   ├── list(ChallengeQuery) -> ReadList<ChallengeProgress>
│   ├── get(ChallengeKey) -> Read<ChallengeProgress>
│   ├── runs(ChallengeRunQuery) -> ReadList<ChallengeRun>
│   └── rewards(ChallengeRewardQuery) -> ReadList<RewardProgress>
│
├── roguelike: RoguelikeApi
│   ├── progression(RogueMode) -> Read<RogueProgress>
│   ├── runs(RogueRunQuery) -> ReadList<RogueRun>
│   └── collection(RogueCollectionQuery) -> ReadList<RogueCollectionEntry>
│
├── battles: BattleHistoryService
│   ├── list(BattleQuery) -> ReadList<BattleReport>
│   ├── get(BattleKey) -> Read<BattleReport>
│   ├── statistics(BattleStatisticsQuery) -> Read<BattleStatistics>
│   └── observe() -> Watch<BattleCompleted>
│
├── gacha: GachaService
│   ├── banners(BannerQuery) -> ReadList<BannerAvailability>
│   ├── history(PullQuery) -> ReadList<PullRecord>
│   ├── observations(LivePullQuery) -> ReadList<LivePullObservation>
│   ├── pity(BannerFamily) -> Read<PityEstimate>
│   └── statistics(GachaStatisticsQuery) -> Read<GachaStatistics>
│
├── shops: ShopService
│   ├── list(ShopQuery) -> ReadList<ShopState>
│   ├── goods(ShopId, GoodsQuery) -> ReadList<GoodsAvailability>
│   └── rewards(ShopId) -> ReadList<RewardProgress>
│
├── collections: CollectionService
│   ├── archive(ArchiveQuery) -> ReadList<ArchiveEntry>
│   ├── monsters(MonsterArchiveQuery) -> ReadList<MonsterArchiveEntry>
│   └── summary() -> Read<CollectionSummary>
│
├── social: SocialApi
│   ├── friends(FriendQuery) -> ReadList<FriendProfile>
│   ├── blocked_players(BlockedPlayerQuery) -> ReadList<PlayerSummary>
│   ├── messages(ChatQuery) -> ReadList<ChatMessage>
│   └── npc_dialogues(DialogueQuery) -> ReadList<DialogueProgress>
│
├── mail: MailService
│   ├── list(MailQuery) -> ReadList<Mail>
│   └── get(MailId) -> Read<Mail>
│
├── journal: AccountJournalService
│   ├── resources(ResourceJournalQuery) -> ReadList<ResourceTransaction>
│   ├── progression(ProgressionJournalQuery) -> ReadList<ProgressionChange>
│   └── farming(FarmingQuery) -> ReadList<FarmingRun>
│
├── snapshot(AccountSectionSet) -> Read<AccountSnapshot>
└── observe(AccountEventFilter) -> Subscription<AccountEvent>
```

Разделы с одним методом можно сначала реализовать простыми сервисами без дополнительных подклассов. Дерево задаёт предметные границы, а не требование создать отдельный crate для каждой строки.

### Профиль и валюты

`PlayerProfile` содержит идентичность и оформление игрока: UID, имя, аватар, уровень и прочие подтверждённые профильные поля. Балансы кредитов, нефрита и энергия в него не вкладываются.

`credits` и `stellar_jade` — отдельные объекты публичного API. Они переиспользуют контракт `CurrencyBalance<C>` и `CurrencyChange<C>`, где параметр `C` фиксирует вид валюты: `Amount<Credits>` нельзя сложить с `Amount<StellarJade>`. Общий `currencies` нужен для перебора всех валют и динамических запросов; он представляет те же наблюдения, не создаёт ещё одно хранилище балансов. `CurrencyBalanceEntry` имеет явный `CurrencyId`.

`CurrencyBalance<C>` хранит текущую сумму. `CurrencyChange<C>` хранит изменение и, только если известно, его причину. История доступна с момента наблюдений/записи; из одного текущего баланса прошлые операции не восстанавливаются. Одна входная команда, обновившая профиль и валюты, применяется атомарно с одной ревизией, хотя публичные сервисы разделены.

## 4. Данные текущей сессии

```text
SessionApi                                   закреплён за SessionId и UID
├── info() -> Read<SessionInfo>
├── world: LiveWorldApi
│   ├── context() -> Read<WorldContext>
│   ├── snapshot(WorldSectionSet) -> Read<WorldSnapshot>
│   ├── floor() -> Read<FloorState>
│   ├── layer() -> Read<MapLayerState>
│   ├── room() -> Read<RoomState>
│   ├── scene() -> Read<SceneState>
│   ├── position() -> Read<PlayerPosition>
│   ├── entities(SceneEntityQuery) -> ReadList<SceneEntity>
│   ├── npcs(NpcQuery) -> ReadList<NpcState>
│   ├── monsters(MonsterQuery) -> ReadList<WorldMonsterState>
│   ├── buffs(SceneBuffQuery) -> ReadList<SceneBuff>
│   ├── transitions(WorldTransitionQuery) -> ReadList<WorldTransition>
│   ├── observe(WorldEventFilter) -> Subscription<WorldEvent>
│   └── observe_position(PositionWatchOptions) -> Watch<PlayerPosition>
└── combat: CombatTelemetryService
    ├── current() -> Read<LiveBattle>
    └── observe(CombatEventFilter) -> Subscription<CombatEvent>
```

Положение в текущей сцене и последний сохранённый `WorldContext` — разные понятия. После разрыва подключения `SessionApi` закрывается; он не продолжает выдавать старую позицию как живую. Ранее сохранённая позиция доступна через наблюдение аккаунта с соответствующими метаданными. `account.world.rooms.current()` означает последнюю наблюдаемую комнату этого аккаунта; `session.world.room()` — комнату подтверждённой активной сессии.

`combat` требует источника подробной телеметрии, например отдельно подключаемого Veritas. Пассивный сетевой источник не обещает поток каждого удара, баффа или изменения HP. Незавершённый бой нельзя автоматически приписывать UID: источник должен иметь подтверждённую привязку к сессии; иначе он остаётся вне аккаунтного состояния и даёт диагностику.

### Пространственная модель мира

Мир представлен несколькими связанными сущностями. Это не обязательная строгая цепочка вложенности: обычная локация может не иметь комнат, а один шаблон комнаты может использоваться в нескольких прохождениях.

| Сущность | Смысл и основные данные |
|---|---|
| `WorldState` / `WorldProgress` | Мир по игровому `world_id`, доступность и наблюдаемый прогресс. Отображаемое название берётся из каталога; равенство `world_id` и пользовательского понятия «планета» проверяется отдельно |
| `PlaneProgress` | Область/локация по `plane_id`, доступные пространства и привязка к миру |
| `FloorState` | Этаж по `floor_id`, контекст измерения/контента, известные слои карты и наблюдаемое состояние |
| `MapLayerState` | Слой карты по `map_layer`, связь с пространством, подпись и высотные границы только при наличии данных |
| `AreaState` | Подзона или область режима: идентификатор, доступность, награды и прочие подтверждённые поля |
| `RoomState` | Идентификатор комнаты, шаблон, тип, статус, положение в прохождении, известные выходы |
| `WorldInstanceState` | Конкретный экземпляр прохождения: режим, ключ запуска, этап/глубина и связанные комнаты |
| `SceneState` | Загруженная сцена: идентификатор, точка входа, лидер, сущности, группы, режим, сюжетный и контентный контексты |
| `ExplorationProgress` | Известные открытые разделы карты, сундуки по типам, телепорты, объекты и покрытие наблюдений |
| `WorldGroupState` | Группа объектов сцены, её состояние и состав |
| `MechanismState` | Типизированные данные распознанного механизма/головоломки; неизвестные состояния сохраняются без выдуманного толкования |
| `WorldTransition` | Наблюдаемый переход между сценами/комнатами/слоями, время и причина, если известна |

`floor_id` — идентификатор определения, а не автоматически число этажа. Отображаемые «1F», «2F», «B1», высота и `map_layer` — разные данные. `dimension_id`, `content_id`, `game_story_line_id` и `game_mode_type` также не сводятся к одному «типу локации».

Контекст упакован в общую модель:

```rust
pub struct WorldContext {
    pub world: Known<WorldId>,
    pub plane: Known<PlaneId>,
    pub space: Known<WorldSpaceKey>,
    pub floor: Known<FloorId>,
    pub map_layer: Known<MapLayerId>,
    pub area: Known<Option<AreaKey>>,
    pub room: Known<Option<RoomKey>>,
    pub instance: Known<Option<InstanceKey>>,
    pub dimension: Known<DimensionId>,
    pub entry: Known<EntryId>,
    pub mode: Known<GameMode>,
    pub content: Known<ContentId>,
    pub story_line: Known<StoryLineId>,
}

pub struct RoomState {
    pub key: RoomKey,
    pub template: Known<RoomTemplateId>,
    pub kind: Known<RoomKind>,
    pub status: Known<RoomStatus>,
    pub details: RoomDetails,
}

pub enum RoomKind {
    Combat,
    Elite,
    Boss,
    Event,
    Rest,
    Transaction,
    Adventure,
    Reward,
    Unknown { mode: GameModeId, code: u32 },
}

pub enum RoomDetails {
    Exploration(ExplorationRoomDetails),
    Roguelike(RogueRoomDetails),
    Challenge(ChallengeRoomDetails),
    Unknown { raw_ref: Option<RawSnapshotRef> },
}
```

Это предлагаемые нормализованные категории комнат, а не доказанный список кодов текущего протокола. `Known::Unknown` означает отсутствие сведений о типе; `RoomKind::Unknown { code, .. }` — полученный, но ещё не распознанный код. Комната «не применима» обозначается `Known(None)` в контексте и `Unavailable(NotApplicable)` для `room()`.

Тип комнаты, вид режима и статус прохождения — разные перечисления. Человекочитаемый тип может поступать из совместимого каталога шаблонов; при отсутствии такого соответствия ID не превращается в название эвристически. `RogueRoomDetails` расширяет общую модель подтверждёнными полями конкретного режима, например ключом прохождения, областью и порядком комнаты, не навязывая их обычному миру.

`RoomKey`, `WorldSpaceKey` и ключи объектов учитывают контекст измерения, сюжетной версии, контента и экземпляра там, где эти значения влияют на идентичность. Одинаковый `room_id` в двух забегах не означает одну комнату. Когда контекст не установлен, адаптер сохраняет непривязанное наблюдение и не сливает его с ранее подтверждённым экземпляром.

`SceneEntity` — размеченное объединение `Actor`, `Npc`, `Monster`, `Prop`, `Summon`, `Unknown`, а не общий мешок полей. У каждой сущности есть общий ключ сцены, положение и типизированные специальные данные. `SceneBuff` описывает только наблюдаемые эффекты сцены и не выдаётся за полный набор боевых баффов.

`WorldSnapshot` содержит контекст и явно выбранные разделы с их доступностью и покрытием. Полный список групп, телепортов или сундуков в одном пакете не делает весь снимок мира полным. Процент исследования вычисляется лишь при известном определении метрики и достоверном знаменателе.

Служебные `floor_saved_data`, `custom_data_list` и неизвестные данные групп не теряются при декодировании. Подтверждённые значения получают типы `WorldFlagState`/`MechanismState`; остальные доступны через ограниченный экспериментальный raw-доступ. Непроверенные флаги не переименовываются в «головоломка решена».

### Основания в локальных схемах протокола

| Схема | Поля, на которые опирается расширение |
|---|---|
| `SceneInfo` | `world_id`, `plane_id`, `floor_id`, `dimension_id`, `entry_id`, `game_mode_type`, `content_id`, `game_story_line_id`, `scene_identifier`, `leader_entity_id`, `entity_list`, `entity_group_list`, `scene_buff_info_list`, `scene_mission_info`, `floor_saved_data`, `custom_data_list` |
| `SceneMapInfo` | `unlocked_teleport_list`, `maze_prop_list`, `maze_group_list`, `chest_list`, `opened_chest_id_list`, `finished_chest_monster_id_list`, контекст сцены и данные миссий |
| `ChestInfo` | Тип сундука, `exist_num`, `opened_num`; семантику счётчиков необходимо подтвердить перед вычислением процентов |
| `EntityMotion`, `SceneActorInfo` | `map_layer`, положение/поворот, данные актёра |
| `ScenePropInfo`, `SceneEntityGroupInfo`, `MazePropState` | Состояние объекта/группы, дополнительные данные, триггеры и времена жизни |
| `EnterRogueMapRoomCsReq`, `PropRogueInfo` | `room_id`; запрос на вход ещё не подтверждает успешный переход |
| `SyncRogueMapRoomScNotify`, вложенный `PAIJCBOKIFO` | `map_id`, `room_id`, статусы `RogueRoomStatus`; часть сопутствующих полей обфусцирована |
| `RogueTournEnterRoomScRsp`, `RogueMagicEnterRoomScRsp` | Вложенные данные сцены при входе; семантика дополнительных полей требует проверки |

Схемы находятся в `Nous-sdk/vendor/reliquary/src/network/command/proto/`. Этот перечень подтверждает наличие структур, но не выполнение живых сценариев и не корректность всех переведённых имён. В частности, смысл `lighten_section_list` нельзя определить только по названию: текущий адаптер уже использует его для списка сундуков по ранее выполненной проверке.

## 5. Каталог: справочники отдельно от аккаунта

```text
GameCatalog
├── characters.get(CharacterId) -> Read<CharacterDefinition>
├── items.get(ItemId) -> Read<ItemDefinition>
├── relic_sets.get(RelicSetId) -> Read<RelicSetDefinition>
├── skills.get(SkillId) -> Read<SkillDefinition>
├── missions.get(MissionId) -> Read<MissionDefinition>
├── achievements.get(AchievementId) -> Read<AchievementDefinition>
├── worlds.get(WorldId) -> Read<WorldDefinition>
├── planes.get(PlaneId) -> Read<PlaneDefinition>
├── floors.get(FloorId) -> Read<FloorDefinition>
├── maps.get(MapDefinitionKey) -> Read<MapDefinition>
├── map_layers.get(MapLayerDefinitionKey) -> Read<MapLayerDefinition>
├── rooms.get(RoomTemplateKey) -> Read<RoomDefinition>
├── room_types.get(RoomTypeKey) -> Read<RoomTypeDefinition>
├── teleports.get(TeleportDefinitionKey) -> Read<TeleportDefinition>
├── activities.schedule(ActivityScheduleQuery) -> ReadList<ActivitySchedule>
└── texts.get(TextKey, Locale) -> Read<LocalizedText>
```

Числовой `ItemId` из трафика не заменяется названием в модели аккаунта. Название, описание, иконка и статические параметры получаются из каталога с указанием версии ресурсов и языка. Неизвестный ID сохраняется, даже если каталог его пока не распознаёт.

Список всех сундуков карты обычно требует справочника; наблюдения открытых сундуков не доказывают полноту карты. `remaining` вычисляется лишь при совместимых версиях каталога и полного прогресса. Назначение координат также различается: позиция игрока при открытии сундука не становится координатой самого сундука.

## 6. Общий тип результата

Ниже концептуальная Rust-запись, не компилируемый патч. Некоторые вспомогательные типы опущены.

```rust
pub type Read<T> = Result<DataState<T>, SdkError>;

pub enum DataState<T> {
    Available(Observation<T>),
    Unavailable(UnavailableData),
}

pub struct Observation<T> {
    pub data: T,
    pub meta: DataMeta,
}

pub struct UnavailableData {
    pub reason: UnavailableReason,
    pub required_action: Option<DataHint>,
}

pub enum UnavailableReason {
    NotObserved,
    Unsupported,
    SourceDisabled,
    SessionEnded,
    NotApplicable,
    NotFound,
}

pub struct DataMeta {
    pub scope: DataScope,             // Account(uid), Session(id, uid), Catalog(version)
    pub version: StateVersion,        // экземпляр хранилища, scope, ревизия
    pub provenance: Vec<Provenance>,  // источник и время его наблюдения
    pub freshness: Freshness,         // Live / Cached / Stale
    pub coverage: Coverage,           // Complete / Partial с причиной
}

pub enum DataSource {
    PassiveTraffic,
    OfficialApi,
    GameResources,
    CombatTelemetry,
    Derived,
}

pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<Cursor>,
    pub total: Option<u64>,           // None, если точное число неизвестно
}
```

- `Available(Page { items: [], ... })` — получена пустая наблюдаемая выборка. `coverage` отдельно говорит, известен ли весь набор.
- `Unavailable(NotObserved)` — данных ещё не было. Подменять это пустым списком нельзя.
- `NotFound` допустим, когда достаточно полное наблюдение позволяет подтвердить отсутствие сущности. Иначе возвращается `NotObserved`.
- `SdkError` — ошибка выполнения: хранилище, неверный фильтр, неподходящий курсор, истёкший снимок. Отсутствие наблюдений само по себе не является поломкой SDK.
- `Freshness::Live` означает данные, подтверждённые текущим источником по правилам конкретного раздела; одно подключение не делает все кешированные разделы живыми.
- Время загрузки файла не подменяет время получения игровых данных. Метаданные обновляются по фактическим источникам и сохраняются вместе с состоянием.
- Для оценки из нескольких источников сохраняются времена и версии входных наблюдений. После отключения источника ранее известные данные могут оставаться `Available` с `Cached`/`Stale`.

Обычные доменные поля не получают обёртку `Observation` каждое. Метаданные находятся на уровне согласованного результата или отдельной сущности, если у элементов разные источники. Частично известные обязательные по смыслу значения используют `Known<T> { Known(T), Unknown }`; `Option<T>` обозначает реальное отсутствие по предметной модели. Обёртки нужны только там, где это различие существенно.

## 7. Предметная типизация и умеренная унификация

### Идентификаторы

Отдельные типы: `AccountId`, `SessionId`, `CharacterId`, `ItemId`, `RelicUid`, `LightConeUid`, `WorldId`, `PlaneId`, `FloorId`, `MapLayerId`, `DimensionId`, `EntryId`, `RoomId`, `RoomTemplateId`, `MissionId`, `AchievementId`. Нельзя передать ID определения реликвии вместо UID экземпляра или ID этажа вместо слоя карты.

Составные ключи используются там, где ID локален: `WorldObjectKey { space, group, config }`, `ChestKey { space, chest_id }`, `RoomKey { space, room_id }`, `ChallengeKey { season, mode, challenge_id }`. `space: WorldSpaceKey` отличает варианты этажа и экземпляры контента. Локальные ID сущностей и боёв дополняются идентификатором сессии/наблюдения, когда протокол не гарантирует глобальную уникальность.

### Время, величины и перечисления

- Временные отметки — `Timestamp` с определённой шкалой UTC; длительности — `Duration`. Не смесь секунд, миллисекунд и строк.
- Серверное локальное время истории прыжков остаётся отдельным типом, пока часовой пояс не установлен. Конверсия явно указывает зону и точность.
- Позиция имеет явные единицы и систему координат. Масштабирование исходных игровых чисел выполняется адаптером без потери исходной точности.
- Количества ресурсов — целочисленные типы; измеренный урон и метрики — валидированные конечные числа. `NaN` и бесконечность не попадают в сохраняемые модели.
- Закрытые статусы — `enum`; игровые коды, которые могут расширяться, имеют `Unknown(raw_code)`. Новое значение не обрушает чтение аккаунта.

### Общие модели там, где общий смысл

`RewardProgress`, `TimeWindow`, `ItemAmount`, `ProgressCounter`, `EquipmentRef`, `Page<T>`, `Observation<T>` переиспользуются. `RewardProgress` отделяет доступность награды от факта получения; неизвестный статус не превращается в `false`.

Реликвия, конус, материал и персонаж сохраняют собственные структуры. Общий `Entity { kind, properties: Map<String, Any> }` для публичного предметного API не используется.

Нестандартные события и режимы представлены размеченными объединениями:

```rust
pub enum EventDetails {
    LoginRewards(LoginRewardProgress),
    Trial(TrialProgress),
    SupportedMinigame(MinigameProgress),
    Unknown { kind: ActivityKindId, raw_ref: Option<RawSnapshotRef> },
}

pub struct EventProgress {
    pub id: ActivityId,
    pub rewards: Vec<RewardProgress>,
    pub details: EventDetails,
}
```

Общие поля дают единый способ работы, специальные детали не теряются. Названия вариантов уточняются при реализации конкретных обработчиков.

### Вычисленные данные

`EnergyForecast`, `PityEstimate`, `FarmingRun` и агрегаты статистики — отдельные модели. Они содержат метод расчёта, ограничения и покрытие входных данных. Неполная история прыжков не даёт достоверный счётчик гаранта; совпадение по предмету и времени не считается надёжным ID для слияния live-наблюдений с импортом.

## 8. Снимки, события и многопоточность

Один `AccountApi.snapshot(sections)` возвращает выбранные разделы из одной ревизии состояния. Отсутствующий раздел помечается как недоступный; одна ревизия не означает, что все разделы были получены от игры одновременно.

Для согласованного обхода больших наборов нужен `ReadView`: первая страница фиксирует ревизию, следующие курсоры привязаны к ней. Курсор другого аккаунта, фильтра или версии отклоняется. Снимки имеют ограниченный срок жизни и лимиты памяти; истёкший курсор возвращает явную ошибку.

Подписка `watch_snapshot(query)` атомарно возвращает начальный снимок и подписку на следующие изменения. Для обычной `observe` определены порядок событий, фильтры и ограниченная очередь. При переполнении — `ResyncRequired`, после которого клиент перечитывает снимок. Медленный потребитель не создаёт бесконечный рост памяти.

Типизированный конверт события содержит scope, ревизию, номер события внутри ревизии и время наблюдения. Одна команда может породить несколько событий с общей ревизией; второе событие нельзя терять только потому, что номер ревизии совпадает.

События различают `Upsert`, `Removed`, `Reset`, `ResyncRequired`. Удаление сущности выводится только из явного удаления или подтверждённого полного снимка. Неполный список не удаляет ненаблюдавшиеся элементы.

Подписки вызывающему приложению не исполняют пользовательские callbacks под lock состояния. Завершение подписки и отключение SDK определены явно. Остановка SDK делает финальный flush и сообщает ошибки записи.

## 9. Источники и возможности

```text
Пакеты / внешний API / ресурсы / телеметрия
                    ↓
        SourceAdapter + SessionBinding
                    ↓
          Validator → DomainCommand
                    ↓
             AccountStateStore
                    ↓
    предметные сервисы → Observation<T>
                    ↓
          приложение-потребитель
```

Внешние типы protobuf остаются в адаптере. Успешность ответа, UID, привязка подключения, длины и диапазоны полей проверяются до изменения состояния.

Основные интерфейсы стратегий: `CaptureSource`, `GameProtocolAdapter`, `AccountStore`, `ResourceProvider`, `CombatTelemetrySource`, `HistoryImporter`. Это маленькие контракты с конкретной ответственностью, без универсального `Provider.execute(name, json)`.

`capabilities.get(DataFeature)` возвращает:

- поддержку: `Supported`, `Partial`, `Unsupported`, `Unknown`;
- доступные источники и их включённость;
- возможности `read`, `observe`, `history`, `import` по отдельности;
- ожидаемое покрытие и поддерживаемую версию протокола;
- типизированную подсказку: например, открыть игровую карту или подключить ресурсный каталог.

Поддержка функции отличается от наличия данных конкретного аккаунта. Источник может поддерживать достижения, хотя этот аккаунт ещё не прислал их список.

## 10. Импорты и диагностический доступ

```text
ImportService
├── gacha_history(HistoryImportRequest) -> ImportJob<HistoryImportReport>
└── recording(RecordingImportRequest) -> ReplayJob<ReplayReport>

DiagnosticsService
├── sources() -> Vec<SourceStatus>
├── observe() -> Subscription<DiagnosticEvent>
└── experimental.raw_snapshots: RawSnapshotService  явно включаемое расширение
```

Импорт истории использует официальный внешний API и отдельную конфигурацию авторизации. Это явная операция с прогрессом и отменой. Обычный `gacha.history()` читает только имеющиеся данные.

Воспроизведение записи по умолчанию идёт в отдельное временное состояние, а не смешивается с живым аккаунтом. Запись старой сессии не подтверждает текущее подключение.

Raw-доступ сохраняет неизвестные данные отдельно от стабильных моделей. Он не обязателен для использования предметного API, имеет лимиты объёма и не раскрывает токены и другие секреты по умолчанию. Произвольный JSON не становится стабильным контрактом SDK.

## 11. Пример использования

Псевдокод в Rust-стиле: показывает предложенную форму, не является работающим примером текущего SDK.

```rust
let sdk = NousSdk::builder()
    .storage(FileAccountStore::new(data_dir))
    .source(PassiveHsrSource::new(capture_config))
    .build()?;

let lifecycle = sdk.observe(SdkEventFilter::Sessions)?;
sdk.runtime().start()?;

// После SessionConfirmed извлекается подтверждённый AccountApi.
let account = sdk.accounts().open(confirmed_account_id)?;

let query = RelicQuery::builder()
    .equipped_by(character_id)
    .min_level(12)
    .page_size(50)
    .build();

let watch = account.inventory().relics().watch_snapshot(query)?;
match watch.initial {
    DataState::Available(result) => {
        for relic in result.data.items {
            println!("{:?}: {}", relic.uid, relic.level);
        }
    }
    DataState::Unavailable(reason) => println!("{:?}", reason),
}

// watch.events: типизированные изменения, reset и запрос перечитывания.
// lifecycle обслуживается отдельно; очереди ограничены.
sdk.runtime().stop()?;
```

## 12. Соотношение с текущей реализацией

| Группа | Текущее основание | Что требуется |
|---|---|---|
| Профиль, кредиты, звёздный нефрит, энергия | Часть полей сейчас объединена в `PlayerProfile` | Разделить публичные сервисы/модели, сохранив атомарность и общую ревизию обновлений; таймер энергии пока пропускается |
| Персонажи, инвентарь | Типизированные модели `nous-core` и адаптер `nous-hsr` | Предметные сервисы и расширение метаданных |
| Мир, этажи, слои, сцены, телепорты, группы, сундуки, объекты | Часть типизированного состояния и более широкие локальные схемы | Расширить модель мира, отделить сессию от кеша, учитывать пространственный контекст и покрытие |
| Комнаты, типы комнат, экземпляры, области и механизмы | Пакеты комнат/режимов и дополнительные данные сцен, часть обфусцирована | Привязать комнаты к прохождениям, проверить поля и источники типов, добавить адаптеры режимов |
| Итоговые бои | Модели статистики и ограниченная история | Сервисы запросов, устойчивые ключи, покрытие и пагинация |
| Баннеры, прыжки | Live-наблюдения и официальный импорт | Сохранить различия источников, явно оценивать полноту истории |
| Отряды, ежедневки, поручения, пропуск, миссии, достижения, почта, друзья | Часть ответов хранится JSON-снимками | Типизированные адаптеры, слияние обновлений, проверка семантики |
| Магазины, архив, расписание и дополнительные события | Есть локальные схемы сообщений | Проверить живые пакеты и значения, добавить обработчики |
| Roguelike | Схемы и отдельные сырые снимки | Разбор конкретных режимов, значительная часть полей обфусцирована |
| Подробная телеметрия боя | Отдельный `nous-veritas` | Опциональный источник и надёжная привязка к сессии |
| Журнал ресурсов, фарм, маршруты, оценки | Производные от наблюдений | Явная запись истории, границы хранения и точности |
| Каталог | Ресурсные механизмы reliquary и ресурсы приложения | Выделить поставщик справочников с версиями и локалями |

Сначала стоит утвердить общий контракт результата, идентификаторы, правила сессии и один вертикальный пример — например, `inventory.relics`. После этого тот же контракт распространяется на остальные категории. Эта последовательность — предложение для будущей реализации; реализация сейчас не выполнялась.
