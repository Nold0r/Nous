use super::*;
use std::sync::atomic::AtomicUsize;

fn frame<M: Message>(id: u16, msg: &M) -> GameCommand {
    GameCommand {
        command_id: id,
        header_len: 0,
        data_len: 0,
        proto_header: vec![],
        proto_data: msg.write_to_bytes().unwrap(),
    }
}
fn login(listener: &TrafficListener, conversation: u32, uid: u32) -> SessionHandle {
    let mut token = proto::PlayerGetTokenScRsp::PlayerGetTokenScRsp::new();
    token.uid = uid;
    listener
        .ingest_command(conversation, &frame(cmd::PlayerGetTokenScRsp, &token))
        .unwrap();
    listener.active_session().unwrap()
}
fn scene(floor: u32) -> Vec<u8> {
    let mut scene = proto::SceneInfo::SceneInfo::new();
    scene.floor_id = floor;
    let mut response = proto::GetCurSceneInfoScRsp::GetCurSceneInfoScRsp::new();
    response.scene = protobuf::MessageField::some(scene);
    response.write_to_bytes().unwrap()
}

#[test]
fn successful_token_confirms_a_session() {
    let listener = TrafficListener::new();
    assert_eq!(login(&listener, 7, 12345).account_uid(), 12345);
    assert!(listener.has_session());
}

#[test]
fn rejected_or_zero_uid_login_does_not_create_a_session() {
    let listener = TrafficListener::new();
    let mut token = proto::PlayerGetTokenScRsp::PlayerGetTokenScRsp::new();
    assert_eq!(
        listener.ingest_command(7, &frame(cmd::PlayerGetTokenScRsp, &token)),
        Err(Error::AuthenticationRejected)
    );
    token.uid = 42;
    token.retcode = 1;
    assert_eq!(
        listener.ingest_command(7, &frame(cmd::PlayerGetTokenScRsp, &token)),
        Err(Error::AuthenticationRejected)
    );
    assert!(!listener.has_session());
}

#[test]
fn reconnect_and_other_conversation_cannot_pollute_the_new_account() {
    let listener = TrafficListener::new();
    let old = login(&listener, 7, 123);
    let current = login(&listener, 8, 456);
    let response = GameCommand {
        command_id: cmd::GetCurSceneInfoScRsp,
        header_len: 0,
        data_len: 0,
        proto_header: vec![],
        proto_data: scene(999),
    };
    assert_eq!(
        listener.ingest_command(7, &response),
        Err(Error::SessionChanged)
    );
    assert!(
        listener
            .read::<GetCurrentScene>(&current)
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        listener.read::<GetCurrentScene>(&old),
        Err(Error::SessionChanged)
    ));
    let renewed = login(&listener, 8, 456);
    assert_ne!(current, renewed);
}

#[test]
fn rejected_response_preserves_the_previous_snapshot() {
    let listener = TrafficListener::new();
    let session = login(&listener, 7, 123);
    listener
        .accept::<GetCurrentScene>(&session, &scene(42))
        .unwrap();
    let mut response = proto::GetCurSceneInfoScRsp::GetCurSceneInfoScRsp::new();
    response.retcode = 9;
    assert!(matches!(
        listener.accept::<GetCurrentScene>(&session, &response.write_to_bytes().unwrap()),
        Err(Error::ResponseRejected(9))
    ));
    assert_eq!(
        listener
            .read::<GetCurrentScene>(&session)
            .unwrap()
            .unwrap()
            .data
            .scene
            .as_ref()
            .unwrap()
            .floor_id,
        42
    );
}

#[test]
fn another_endpoint_with_the_same_conversation_cannot_update_or_disconnect_the_session() {
    let listener = TrafficListener::new();
    let mut token = proto::PlayerGetTokenScRsp::PlayerGetTokenScRsp::new();
    token.uid = 123;
    listener
        .ingest_on_flow(10, 7, &frame(cmd::PlayerGetTokenScRsp, &token))
        .unwrap();
    let session = listener.active_session().unwrap();
    let response = GameCommand {
        command_id: cmd::GetCurSceneInfoScRsp,
        header_len: 0,
        data_len: 0,
        proto_header: vec![],
        proto_data: scene(42),
    };
    assert_eq!(
        listener.ingest_on_flow(11, 7, &response),
        Err(Error::SessionChanged)
    );
    listener.invalidate_flow(11);
    assert_eq!(listener.active_session(), Some(session));
}

#[test]
fn failed_login_on_another_flow_does_not_drop_a_valid_session() {
    let listener = TrafficListener::new();
    let current = login(&listener, 7, 123);
    let mut token = proto::PlayerGetTokenScRsp::PlayerGetTokenScRsp::new();
    token.uid = 456;
    token.retcode = 1;
    assert_eq!(
        listener.ingest_on_flow(99, 7, &frame(cmd::PlayerGetTokenScRsp, &token)),
        Err(Error::AuthenticationRejected)
    );
    assert_eq!(listener.active_session(), Some(current));
}

struct TestTransport {
    bound: SessionHandle,
    calls: AtomicUsize,
    delay: Duration,
    wrong_correlation: bool,
    wrong_command: bool,
    invalidate: Option<TrafficListener>,
}
impl TestTransport {
    fn new(bound: SessionHandle) -> Self {
        Self {
            bound,
            calls: AtomicUsize::new(0),
            delay: Duration::ZERO,
            wrong_correlation: false,
            wrong_command: false,
            invalidate: None,
        }
    }
}
impl SessionTransport for TestTransport {
    fn bound_session(&self) -> SessionHandle {
        self.bound.clone()
    }
    fn round_trip(&self, request: RequestEnvelope) -> TransportFuture<'_> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(request.command_id, cmd::GetCurSceneInfoCsReq);
            assert!(request.body.is_empty());
            tokio::time::sleep(self.delay).await;
            if let Some(listener) = &self.invalidate {
                listener.invalidate();
            }
            Ok(ReplyEnvelope {
                session: request.session,
                request_id: request.request_id + u64::from(self.wrong_correlation),
                command_id: if self.wrong_command {
                    cmd::GetBagScRsp
                } else {
                    request.expected_response_id
                },
                body: scene(314),
            })
        })
    }
}

#[tokio::test]
async fn passive_capture_is_not_a_sending_transport() {
    let listener = TrafficListener::new();
    let session = login(&listener, 7, 123);
    let requester = SessionRequester::new(listener);
    assert!(matches!(
        requester
            .query(&session, GetCurrentScene, Duration::from_secs(1))
            .await,
        Err(Error::TransportUnavailable)
    ));
}

#[tokio::test]
async fn replay_and_wrong_transport_never_send() {
    let listener = TrafficListener::replay();
    let session = login(&listener, 7, 123);
    let transport = Arc::new(TestTransport::new(session.clone()));
    let requester = SessionRequester::with_transport(listener, transport.clone());
    assert!(matches!(
        requester
            .query(&session, GetCurrentScene, Duration::from_secs(1))
            .await,
        Err(Error::ReplayOnly)
    ));
    assert_eq!(transport.calls.load(Ordering::SeqCst), 0);

    let listener = TrafficListener::new();
    let session = login(&listener, 7, 123);
    let other = TrafficListener::new();
    let other_session = login(&other, 7, 123);
    let transport = Arc::new(TestTransport::new(other_session));
    let requester = SessionRequester::with_transport(listener, transport.clone());
    assert!(matches!(
        requester
            .query(&session, GetCurrentScene, Duration::from_secs(1))
            .await,
        Err(Error::WrongTransportSession)
    ));
    assert_eq!(transport.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn typed_reply_updates_the_same_state() {
    let listener = TrafficListener::new();
    let session = login(&listener, 7, 123);
    let requester = SessionRequester::with_transport(
        listener.clone(),
        Arc::new(TestTransport::new(session.clone())),
    );
    let result = requester
        .query(&session, GetCurrentScene, Duration::from_secs(1))
        .await
        .unwrap();
    let cached = listener.read::<GetCurrentScene>(&session).unwrap().unwrap();
    assert_eq!(result.revision, cached.revision);
    assert_eq!(cached.data.scene.as_ref().unwrap().floor_id, 314);
}

#[tokio::test]
async fn unrelated_replies_are_not_accepted() {
    for wrong_command in [false, true] {
        let listener = TrafficListener::new();
        let session = login(&listener, 7, 123);
        let mut transport = TestTransport::new(session.clone());
        transport.wrong_command = wrong_command;
        transport.wrong_correlation = !wrong_command;
        let requester = SessionRequester::with_transport(listener.clone(), Arc::new(transport));
        assert!(matches!(
            requester
                .query(&session, GetCurrentScene, Duration::from_secs(1))
                .await,
            Err(Error::WrongResponse)
        ));
        assert!(
            listener
                .read::<GetCurrentScene>(&session)
                .unwrap()
                .is_none()
        );
    }
}

#[tokio::test]
async fn timeout_and_disconnect_cannot_publish_a_reply() {
    let listener = TrafficListener::new();
    let session = login(&listener, 7, 123);
    let mut transport = TestTransport::new(session.clone());
    transport.delay = Duration::from_secs(5);
    let requester = SessionRequester::with_transport(listener.clone(), Arc::new(transport));
    assert!(matches!(
        requester
            .query(&session, GetCurrentScene, Duration::from_millis(5))
            .await,
        Err(Error::Timeout)
    ));
    assert!(
        listener
            .read::<GetCurrentScene>(&session)
            .unwrap()
            .is_none()
    );

    let mut transport = TestTransport::new(session.clone());
    transport.invalidate = Some(listener.clone());
    let requester = SessionRequester::with_transport(listener, Arc::new(transport));
    assert!(matches!(
        requester
            .query(&session, GetCurrentScene, Duration::from_secs(1))
            .await,
        Err(Error::SessionChanged)
    ));
}
