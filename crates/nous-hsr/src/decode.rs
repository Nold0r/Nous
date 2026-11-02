use nous_core::game::{Command, ProfilePatch};
use reliquary::network::command::{GameCommand, command_id as cmd, proto};

/// Converts a decrypted wire packet into a protocol-free core command.
/// Unknown/malformed packets produce no domain changes, as before.
pub fn decode(command: &GameCommand) -> Command {
    let mut out = Command::default();
    match command.command_id {
        cmd::PlayerGetTokenScRsp => {
            if let Ok(r) = command.parse_proto::<proto::PlayerGetTokenScRsp::PlayerGetTokenScRsp>()
            {
                out.login = Some(r.uid);
            }
        }
        cmd::PlayerLoginScRsp => {
            if let Ok(r) = command.parse_proto::<proto::PlayerLoginScRsp::PlayerLoginScRsp>() {
                out.profile = Some(r.basic_info.as_ref().map(basic).unwrap_or_default());
            }
        }
        cmd::PlayerSyncScNotify => {
            if let Ok(r) = command.parse_proto::<proto::PlayerSyncScNotify::PlayerSyncScNotify>() {
                out.profile = r.basic_info.as_ref().map(basic);
                out.inventory_sync = Some(crate::inventory::sync(&r));
            }
        }
        cmd::StaminaInfoScNotify => {
            if let Ok(r) = command.parse_proto::<proto::StaminaInfoScNotify::StaminaInfoScNotify>()
            {
                out.profile = Some(ProfilePatch {
                    stamina: Some(r.stamina),
                    reserve_stamina: Some(r.reserve_stamina),
                    ..Default::default()
                });
            }
        }
        cmd::GetPlayerBoardDataScRsp => {
            if let Ok(r) =
                command.parse_proto::<proto::GetPlayerBoardDataScRsp::GetPlayerBoardDataScRsp>()
            {
                if r.current_head_icon_id != 0 {
                    out.profile = Some(ProfilePatch {
                        avatar: Some(r.current_head_icon_id),
                        ..Default::default()
                    });
                }
            }
        }
        cmd::GetBagScRsp => {
            if let Ok(r) = command.parse_proto::<proto::GetBagScRsp::GetBagScRsp>() {
                out.bag = Some(crate::inventory::bag(&r));
            }
        }
        cmd::GetAvatarDataScRsp => {
            if let Ok(r) = command.parse_proto::<proto::GetAvatarDataScRsp::GetAvatarDataScRsp>() {
                out.characters = Some(crate::inventory::characters(
                    &r.avatar_list,
                    &r.avatar_path_data_info_list,
                    r.is_get_all,
                ));
            }
        }
        _ => {}
    }
    let now = nous_core::profile::now();
    out.world = crate::world::decode(command);
    out.battle = crate::battles::decode(command, now);
    match crate::gacha::decode(command, now) {
        Some(crate::gacha::Update::Banners(banners)) => out.banners = Some(banners),
        Some(crate::gacha::Update::Pulls(pulls)) => out.pulls = Some(pulls),
        None => {}
    }
    let mut snapshots = nous_core::snapshots::Snapshots::default();
    crate::snapshots::handle(&mut snapshots, command, now);
    out.snapshot = snapshots.into_iter().next();
    out
}
fn basic(p: &proto::PlayerBasicInfo::PlayerBasicInfo) -> ProfilePatch {
    ProfilePatch {
        nickname: (!p.nickname.is_empty()).then(|| p.nickname.clone()),
        level: (p.level != 0).then_some(p.level),
        stamina: Some(p.stamina),
        jade: Some(p.stellar_jade_count as u64),
        credits: Some(p.credit_count as u64),
        ..Default::default()
    }
}
