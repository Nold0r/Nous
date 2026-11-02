//! Raw snapshots: the latest answer of the game to informative requests, kept
//! whole as JSON (protobuf field names; some are obfuscated, e.g.
//! `CJLCPMDGIBO`). A cheap way to expose everything the traffic carries before
//! a feature models it properly, as inventory.rs or battles.rs do.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// missing fields (older files) take defaults: adding a field never breaks loading
#[serde(rename_all = "camelCase", default)]
pub struct Snapshot {
    /// Unix seconds Nous saw it.
    pub at: i64,
    pub data: serde_json::Value,
}

/// Command name -> latest snapshot.
pub type Snapshots = BTreeMap<String, Snapshot>;

/// What each kept command means, for the docs and the UI.
pub const KINDS: &[(&str, &str)] = &[
    (
        "GetDailyActiveInfoScRsp",
        "ежедневная тренировка: очки активности, задания",
    ),
    (
        "GetExpeditionDataScRsp",
        "поручения: кто и куда отправлен, когда вернутся",
    ),
    ("ExpeditionDataChangeScNotify", "изменение поручений"),
    (
        "GetChallengeScRsp",
        "Зал забвения, Чистый вымысел, Иллюзия конца: этапы и звёзды",
    ),
    ("GetChallengePeakDataScRsp", "пиковые испытания"),
    ("GetAllLineupDataScRsp", "все отряды"),
    ("GetCurLineupDataScRsp", "текущий отряд"),
    ("BattlePassInfoNotify", "Безымянная слава: уровень и опыт"),
    ("GetRogueInfoScRsp", "Виртуальная вселенная"),
    ("RogueTournQueryScRsp", "Расходящаяся вселенная"),
    ("GetMissionDataScRsp", "задания (миссии)"),
    (
        "GetQuestDataScRsp",
        "достижения, ежедневные и прочие задачи",
    ),
    ("GetMailScRsp", "почта"),
    ("GetFriendListInfoScRsp", "друзья"),
    ("GetMarkChestScRsp", "отмеченные сундуки"),
    ("GetBattleCollegeDataScRsp", "Обучение боям"),
    ("GetTrainVisitorRegisterScRsp", "гости Экспресса"),
];
