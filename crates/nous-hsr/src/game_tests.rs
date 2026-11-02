mod tests {
    use nous_core::account::FileStore as Store;
    use nous_core::game::GameState;
    use reliquary::network::command::{GameCommand, command_id as cmd, proto};

    fn command<M: protobuf::Message>(id: u16, msg: &M) -> GameCommand {
        let data = msg.write_to_bytes().unwrap();
        GameCommand {
            command_id: id,
            header_len: 0,
            data_len: data.len() as u32,
            proto_header: vec![],
            proto_data: data,
        }
    }

    fn token(uid: u32) -> GameCommand {
        let mut t = proto::PlayerGetTokenScRsp::PlayerGetTokenScRsp::new();
        t.uid = uid;
        command(cmd::PlayerGetTokenScRsp, &t)
    }

    fn bag_with_relic(relic_uid: u32) -> GameCommand {
        let mut r = proto::Relic::Relic::new();
        r.unique_id = relic_uid;
        r.tid = 61011;
        r.main_affix_id = 1;
        let mut b = proto::GetBagScRsp::GetBagScRsp::new();
        b.relic_list = vec![r];
        command(cmd::GetBagScRsp, &b)
    }

    fn battle() -> GameCommand {
        let mut req = proto::PVEBattleResultCsReq::PVEBattleResultCsReq::new();
        req.stage_id = 1;
        command(cmd::PVEBattleResultCsReq, &req)
    }

    fn temp_store(name: &str) -> (Store, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("nous-game-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        (Store::new(Some(dir.clone())), dir)
    }

    #[test]
    fn switching_account_replaces_everything_and_keeps_the_old_one() {
        let (store, dir) = temp_store("switch");
        let mut game = GameState::new(store.clone()).unwrap();

        let ch = game.handle(&crate::decode(&token(1))).unwrap();
        assert!(ch.account_switched && ch.profile);
        game.handle(&crate::decode(&bag_with_relic(10))).unwrap();
        let ch = game.handle(&crate::decode(&battle())).unwrap();
        assert!(ch.battle.is_some());
        let rev_a = ch.revision;

        let ch = game.handle(&crate::decode(&token(2))).unwrap();
        assert!(
            ch.account_switched && ch.inventory_reset,
            "consumers must drop account 1"
        );
        assert!(ch.revision > rev_a, "revisions only grow");
        assert_eq!(game.account.uid, Some(2));
        assert!(game.account.inventory.relics.is_empty());
        assert!(
            game.account.battles.list.is_empty(),
            "battles of account 1 don't leak into 2"
        );

        // account 1 was written before the switch, with its relic and battle
        let mut back = GameState::new(store.clone()).unwrap();
        back.handle(&crate::decode(&token(1))).unwrap();
        assert!(back.account.inventory.relics.contains_key(&10));
        assert_eq!(back.account.battles.list.len(), 1);

        // same uid again: not a switch
        assert!(
            !back
                .handle(&crate::decode(&token(1)))
                .unwrap()
                .account_switched
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn handling_marks_unsaved_instead_of_writing() {
        let (store, dir) = temp_store("dirty");
        let mut game = GameState::new(store).unwrap();
        game.handle(&crate::decode(&token(3))).unwrap();
        let before = game.revision();
        game.handle(&crate::decode(&bag_with_relic(1))).unwrap();
        assert!(game.revision() > before);
        assert!(game.take_unsaved().is_some(), "changed: one copy to write");
        assert!(game.take_unsaved().is_none(), "nothing new since");
        std::fs::remove_dir_all(dir).ok();
    }
}
