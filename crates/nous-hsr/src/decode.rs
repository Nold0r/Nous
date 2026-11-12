use nous_core::game::{Command, ProfilePatch};
use reliquary::network::command::{GameCommand, command_id as cmd, proto};

/// Converts a decrypted wire packet into a protocol-free core command.
/// Unknown/malformed packets produce no domain changes, as before.
pub fn decode(command: &GameCommand) -> Command {
    let mut out = Command::default();
    match command.command_id {
        cmd::PlayerGetTokenScRsp => {
            if let Some(r) = rsp!(command, PlayerGetTokenScRsp)
                && r.uid != 0
            {
                out.login = Some(r.uid);
            }
        }
        cmd::PlayerLoginScRsp => {
            if let Some(r) = rsp!(command, PlayerLoginScRsp) {
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
            if let Some(r) = rsp!(command, GetPlayerBoardDataScRsp) {
                if r.current_head_icon_id != 0 {
                    out.profile = Some(ProfilePatch {
                        avatar: Some(r.current_head_icon_id),
                        ..Default::default()
                    });
                }
            }
        }
        cmd::GetBagScRsp => {
            if let Some(r) = rsp!(command, GetBagScRsp) {
                out.bag = Some(crate::inventory::bag(&r));
            }
        }
        cmd::GetAvatarDataScRsp => {
            if let Some(r) = rsp!(command, GetAvatarDataScRsp) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use protobuf::Message;

    fn command(command_id: u16, message: &impl Message) -> GameCommand {
        GameCommand {
            command_id,
            header_len: 0,
            data_len: 0,
            proto_header: vec![],
            proto_data: message.write_to_bytes().unwrap(),
        }
    }

    #[test]
    fn a_refused_login_is_not_a_login() {
        let mut token = proto::PlayerGetTokenScRsp::PlayerGetTokenScRsp::new();
        token.retcode = 1003;
        token.uid = 0;
        assert_eq!(decode(&command(cmd::PlayerGetTokenScRsp, &token)).login, None, "refused");
        token.uid = 42;
        assert_eq!(decode(&command(cmd::PlayerGetTokenScRsp, &token)).login, None, "refused with a uid");
        token.retcode = 0;
        token.uid = 0;
        assert_eq!(decode(&command(cmd::PlayerGetTokenScRsp, &token)).login, None, "no account 0");
        token.uid = 42;
        assert_eq!(decode(&command(cmd::PlayerGetTokenScRsp, &token)).login, Some(42));
    }

    #[test]
    fn a_refused_answer_does_not_replace_stored_data() {
        let mut bag = proto::GetBagScRsp::GetBagScRsp::new();
        bag.retcode = 1;
        assert!(decode(&command(cmd::GetBagScRsp, &bag)).bag.is_none(), "an empty bag would wipe the inventory");
        bag.retcode = 0;
        assert!(decode(&command(cmd::GetBagScRsp, &bag)).bag.is_some());

        let mut avatars = proto::GetAvatarDataScRsp::GetAvatarDataScRsp::new();
        avatars.retcode = 1;
        avatars.is_get_all = true;
        assert!(decode(&command(cmd::GetAvatarDataScRsp, &avatars)).characters.is_none());

        let mut map = proto::GetSceneMapInfoScRsp::GetSceneMapInfoScRsp::new();
        map.retcode = 1;
        assert!(decode(&command(cmd::GetSceneMapInfoScRsp, &map)).world.is_none());

        let mut banners = proto::GetGachaInfoScRsp::GetGachaInfoScRsp::new();
        banners.retcode = 1;
        assert!(decode(&command(cmd::GetGachaInfoScRsp, &banners)).banners.is_none());

        let mut mail = proto::GetMailScRsp::GetMailScRsp::new();
        mail.retcode = 1;
        assert!(decode(&command(cmd::GetMailScRsp, &mail)).snapshot.is_none());
        mail.retcode = 0;
        assert!(decode(&command(cmd::GetMailScRsp, &mail)).snapshot.is_some());
    }
}
