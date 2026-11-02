//! Raw snapshots: the latest answer of the game to informative requests, kept
//! whole as JSON (protobuf field names; some are obfuscated, e.g.
//! `CJLCPMDGIBO`). A cheap way to expose everything the traffic carries before
//! a feature models it properly, as inventory.rs or battles.rs do.

use std::collections::BTreeMap;

use reliquary::network::command::GameCommand;
use reliquary::network::command::command_id as cmd;
use reliquary::network::command::proto;
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
    ("GetDailyActiveInfoScRsp", "ежедневная тренировка: очки активности, задания"),
    ("GetExpeditionDataScRsp", "поручения: кто и куда отправлен, когда вернутся"),
    ("ExpeditionDataChangeScNotify", "изменение поручений"),
    ("GetChallengeScRsp", "Зал забвения, Чистый вымысел, Иллюзия конца: этапы и звёзды"),
    ("GetChallengePeakDataScRsp", "пиковые испытания"),
    ("GetAllLineupDataScRsp", "все отряды"),
    ("GetCurLineupDataScRsp", "текущий отряд"),
    ("BattlePassInfoNotify", "Безымянная слава: уровень и опыт"),
    ("GetRogueInfoScRsp", "Виртуальная вселенная"),
    ("RogueTournQueryScRsp", "Расходящаяся вселенная"),
    ("GetMissionDataScRsp", "задания (миссии)"),
    ("GetQuestDataScRsp", "достижения, ежедневные и прочие задачи"),
    ("GetMailScRsp", "почта"),
    ("GetFriendListInfoScRsp", "друзья"),
    ("GetMarkChestScRsp", "отмеченные сундуки"),
    ("GetBattleCollegeDataScRsp", "Обучение боям"),
    ("GetTrainVisitorRegisterScRsp", "гости Экспресса"),
];

/// Stores the command if it's one of KINDS; true if it was.
pub fn handle(snapshots: &mut Snapshots, command: &GameCommand, now: i64) -> bool {
    macro_rules! keep {
        ($($ty:ident),+ $(,)?) => {
            match command.command_id {
                $(cmd::$ty => command.parse_proto::<proto::$ty::$ty>().ok().and_then(|m| to_json(&m)),)+
                _ => return false,
            }
        };
    }
    let data = keep!(
        GetDailyActiveInfoScRsp,
        GetExpeditionDataScRsp,
        ExpeditionDataChangeScNotify,
        GetChallengeScRsp,
        GetChallengePeakDataScRsp,
        GetAllLineupDataScRsp,
        GetCurLineupDataScRsp,
        BattlePassInfoNotify,
        GetRogueInfoScRsp,
        RogueTournQueryScRsp,
        GetMissionDataScRsp,
        GetQuestDataScRsp,
        GetMailScRsp,
        GetFriendListInfoScRsp,
        GetMarkChestScRsp,
        GetBattleCollegeDataScRsp,
        GetTrainVisitorRegisterScRsp,
    );
    let (Some(data), Some(name)) = (data, command.get_command_name()) else { return false };
    snapshots.insert(name.to_string(), Snapshot { at: now, data });
    true
}

/// Which raw snapshots exist and what they hold: [name, description].
#[tauri::command]
pub fn get_snapshot_kinds() -> Vec<(&'static str, &'static str)> {
    KINDS.to_vec()
}

fn to_json<M: protobuf::MessageFull>(msg: &M) -> Option<serde_json::Value> {
    let text = protobuf_json_mapping::print_to_string(msg).ok()?;
    serde_json::from_str(&text).ok()
}

