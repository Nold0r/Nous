use nous_core::snapshots::{Snapshot, Snapshots};
use reliquary::network::command::{GameCommand, command_id as cmd, proto};
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
    let (Some(data), Some(name)) = (data, command.get_command_name()) else {
        return false;
    };
    snapshots.insert(name.to_string(), Snapshot { at: now, data });
    true
}

fn to_json<M: protobuf::MessageFull>(msg: &M) -> Option<serde_json::Value> {
    let text = protobuf_json_mapping::print_to_string(msg).ok()?;
    serde_json::from_str(&text).ok()
}
