//! Experimental two-object API. No official-game sending transport is bundled.
//! The passive capture/replay path never pretends to own the client's socket.
use protobuf::{Message, MessageFull};
use reliquary::network::command::{GameCommand, command_id as cmd, proto};
use std::{
    collections::HashMap,
    fmt,
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

static INSTANCE: AtomicU64 = AtomicU64::new(1);
static REQUEST: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    PassiveCapture,
    Replay,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SessionHandle {
    instance: u64,
    epoch: u64,
    conversation: u32,
    flow: u64,
    uid: u32,
    origin: Origin,
}
impl SessionHandle {
    pub fn account_uid(&self) -> u32 {
        self.uid
    }
    pub fn origin(&self) -> Origin {
        self.origin
    }
}
impl fmt::Debug for SessionHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionHandle")
            .field("epoch", &self.epoch)
            .field("origin", &self.origin)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    NoSession,
    SessionChanged,
    AuthenticationRejected,
    ReplayOnly,
    TransportUnavailable,
    WrongTransportSession,
    WrongResponse,
    Timeout,
    MalformedPacket,
    ResponseRejected(u32),
    Transport(String),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

pub struct Observation<T> {
    pub session: SessionHandle,
    pub revision: u64,
    pub data: T,
}

struct State {
    session: Option<SessionHandle>,
    epoch: u64,
    revision: u64,
    responses: HashMap<u16, (u64, Vec<u8>)>,
}

#[derive(Clone)]
pub struct TrafficListener {
    instance: u64,
    origin: Origin,
    state: Arc<Mutex<State>>,
}
impl Default for TrafficListener {
    fn default() -> Self {
        Self::new()
    }
}
impl TrafficListener {
    pub fn new() -> Self {
        Self::with_origin(Origin::PassiveCapture)
    }
    pub fn replay() -> Self {
        Self::with_origin(Origin::Replay)
    }
    fn with_origin(origin: Origin) -> Self {
        Self {
            instance: INSTANCE.fetch_add(1, Ordering::Relaxed),
            origin,
            state: Arc::new(Mutex::new(State {
                session: None,
                epoch: 0,
                revision: 0,
                responses: HashMap::new(),
            })),
        }
    }
    pub fn active_session(&self) -> Option<SessionHandle> {
        self.state.lock().unwrap().session.clone()
    }
    pub fn has_session(&self) -> bool {
        self.active_session().is_some()
    }
    pub fn invalidate(&self) {
        let mut state = self.state.lock().unwrap();
        state.session = None;
        state.responses.clear();
    }
    pub fn invalidate_flow(&self, flow: u64) {
        let mut state = self.state.lock().unwrap();
        if state.session.as_ref().is_some_and(|s| s.flow == flow) {
            state.session = None;
            state.responses.clear();
        }
    }
    pub fn ingest_command(&self, conversation: u32, command: &GameCommand) -> Result<(), Error> {
        self.ingest_on_flow(0, conversation, command)
    }
    /// A flow ID must identify one local/remote endpoint tuple within this listener.
    pub fn ingest_on_flow(
        &self,
        flow: u64,
        conversation: u32,
        command: &GameCommand,
    ) -> Result<(), Error> {
        if command.command_id == cmd::PlayerGetTokenScRsp {
            let token =
                match command.parse_proto::<proto::PlayerGetTokenScRsp::PlayerGetTokenScRsp>() {
                    Ok(token) => token,
                    Err(_) => {
                        self.invalidate_flow(flow);
                        return Err(Error::MalformedPacket);
                    }
                };
            if token.retcode != 0 || token.uid == 0 {
                self.invalidate_flow(flow);
                return Err(Error::AuthenticationRejected);
            }
            let mut state = self.state.lock().unwrap();
            state.session = None;
            state.responses.clear();
            state.epoch += 1;
            state.session = Some(SessionHandle {
                instance: self.instance,
                epoch: state.epoch,
                conversation,
                flow,
                uid: token.uid,
                origin: self.origin,
            });
            return Ok(());
        }
        let session = self.active_session().ok_or(Error::NoSession)?;
        if session.conversation != conversation || session.flow != flow {
            return Err(Error::SessionChanged);
        }
        match command.command_id {
            cmd::GetCurSceneInfoScRsp => self
                .accept::<GetCurrentScene>(&session, &command.proto_data)
                .map(|_| ()),
            cmd::GetBagScRsp => self
                .accept::<GetInventory>(&session, &command.proto_data)
                .map(|_| ()),
            _ => Ok(()),
        }
    }
    pub fn read<Q: ReadQuery>(
        &self,
        session: &SessionHandle,
    ) -> Result<Option<Observation<Q::Response>>, Error> {
        let state = self.state.lock().unwrap();
        Self::check(&state, session)?;
        let Some((revision, bytes)) = state.responses.get(&Q::RESPONSE_ID) else {
            return Ok(None);
        };
        let data = Q::decode(bytes)?;
        Ok(Some(Observation {
            session: session.clone(),
            revision: *revision,
            data,
        }))
    }
    fn check(state: &State, session: &SessionHandle) -> Result<(), Error> {
        if state.session.as_ref() == Some(session) {
            Ok(())
        } else {
            Err(Error::SessionChanged)
        }
    }
    fn accept<Q: ReadQuery>(
        &self,
        session: &SessionHandle,
        bytes: &[u8],
    ) -> Result<Observation<Q::Response>, Error> {
        let data = Q::decode(bytes)?;
        let mut state = self.state.lock().unwrap();
        Self::check(&state, session)?;
        state.revision += 1;
        let revision = state.revision;
        state
            .responses
            .insert(Q::RESPONSE_ID, (revision, bytes.to_vec()));
        Ok(Observation {
            session: session.clone(),
            revision,
            data,
        })
    }
}

mod sealed {
    pub trait Sealed {}
}
/// A small, sealed query surface, not permission to send arbitrary command IDs.
/// These names describe the expected operation; official-side effects are not validated.
pub trait ReadQuery: sealed::Sealed + Send {
    type Response: MessageFull;
    const REQUEST_ID: u16;
    const RESPONSE_ID: u16;
    fn encode(&self) -> Vec<u8>;
    fn decode(bytes: &[u8]) -> Result<Self::Response, Error>;
}
pub struct GetCurrentScene;
pub struct GetInventory;
macro_rules! query {
    ($query:ident, $request:ident, $response:ident) => {
        impl sealed::Sealed for $query {}
        impl ReadQuery for $query {
            type Response = proto::$response::$response;
            const REQUEST_ID: u16 = cmd::$request;
            const RESPONSE_ID: u16 = cmd::$response;
            fn encode(&self) -> Vec<u8> {
                Vec::new()
            } // Both request messages are empty in this schema.
            fn decode(bytes: &[u8]) -> Result<Self::Response, Error> {
                let data =
                    Self::Response::parse_from_bytes(bytes).map_err(|_| Error::MalformedPacket)?;
                if data.retcode != 0 {
                    return Err(Error::ResponseRejected(data.retcode));
                }
                Ok(data)
            }
        }
    };
}
query!(GetCurrentScene, GetCurSceneInfoCsReq, GetCurSceneInfoScRsp);
query!(GetInventory, GetBagCsReq, GetBagScRsp);

pub struct RequestEnvelope {
    pub session: SessionHandle,
    pub request_id: u64,
    pub command_id: u16,
    pub expected_response_id: u16,
    pub body: Vec<u8>,
}
pub struct ReplyEnvelope {
    pub session: SessionHandle,
    pub request_id: u64,
    pub command_id: u16,
    pub body: Vec<u8>,
}
pub type TransportFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ReplyEnvelope, Error>> + Send + 'a>>;
/// Trusted integration boundary. The implementation must own/synchronize the
/// real transport and correlate responses, including requests made by the game.
/// Captured key/conv values alone do not meet this contract.
pub trait SessionTransport: Send + Sync {
    fn bound_session(&self) -> SessionHandle;
    fn round_trip(&self, request: RequestEnvelope) -> TransportFuture<'_>;
}

pub struct SessionRequester {
    listener: TrafficListener,
    transport: Option<Arc<dyn SessionTransport>>,
}
impl SessionRequester {
    pub fn new(listener: TrafficListener) -> Self {
        Self {
            listener,
            transport: None,
        }
    }
    /// Caller-supplied cooperative connection. No implementation for the official client's KCP is included.
    pub fn with_transport(listener: TrafficListener, transport: Arc<dyn SessionTransport>) -> Self {
        Self {
            listener,
            transport: Some(transport),
        }
    }
    pub async fn query<Q: ReadQuery>(
        &self,
        session: &SessionHandle,
        query: Q,
        timeout: Duration,
    ) -> Result<Observation<Q::Response>, Error> {
        {
            let state = self.listener.state.lock().unwrap();
            TrafficListener::check(&state, session)?;
        }
        if session.origin == Origin::Replay {
            return Err(Error::ReplayOnly);
        }
        let transport = self.transport.as_ref().ok_or(Error::TransportUnavailable)?;
        if transport.bound_session() != *session {
            return Err(Error::WrongTransportSession);
        }
        let request_id = REQUEST.fetch_add(1, Ordering::Relaxed);
        let request = RequestEnvelope {
            session: session.clone(),
            request_id,
            command_id: Q::REQUEST_ID,
            expected_response_id: Q::RESPONSE_ID,
            body: query.encode(),
        };
        let reply = tokio::time::timeout(timeout, transport.round_trip(request))
            .await
            .map_err(|_| Error::Timeout)??;
        if reply.session != *session
            || reply.request_id != request_id
            || reply.command_id != Q::RESPONSE_ID
        {
            return Err(Error::WrongResponse);
        }
        self.listener.accept::<Q>(session, &reply.body)
    }
}

#[cfg(test)]
mod tests;
