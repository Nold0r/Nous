use crate::{Account, AccountStore, Command, Event, GameState, Versioned};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

struct Inner {
    game: Mutex<GameState>,
    subscribers: Mutex<Vec<mpsc::Sender<Event>>>,
}
/// Cloneable state/event client. The last owner stops and joins the saver,
/// flushes pending writes and releases subscriptions. Subscribe before reading.
#[derive(Clone)]
pub struct Client {
    owner: Arc<Owner>,
}
struct Owner {
    inner: Arc<Inner>,
    stop: mpsc::Sender<()>,
    saver: Mutex<Option<JoinHandle<()>>>,
}
impl Client {
    pub fn new(store: impl AccountStore + 'static) -> Result<Self, String> {
        Self::with_store(Arc::new(store))
    }
    pub fn with_store(store: Arc<dyn AccountStore>) -> Result<Self, String> {
        let inner = Arc::new(Inner {
            game: Mutex::new(GameState::with_store(store)?),
            subscribers: Mutex::new(Vec::new()),
        });
        let (stop, stopped) = mpsc::channel();
        let worker = inner.clone();
        let saver = std::thread::Builder::new()
            .name("nous-saver".into())
            .spawn(move || {
                let mut ticks = 0u32;
                while stopped.recv_timeout(TICK) == Err(mpsc::RecvTimeoutError::Timeout) {
                    worker.tick();
                    ticks += 1;
                    if ticks % SAVE_EVERY_TICKS == 0
                        && let Err(e) = worker.flush()
                    {
                        worker.publish(Event::error(e));
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            owner: Arc::new(Owner {
                inner,
                stop,
                saver: Mutex::new(Some(saver)),
            }),
        })
    }
    pub fn subscribe(&self) -> mpsc::Receiver<Event> {
        let (tx, rx) = mpsc::channel();
        self.owner.inner.subscribers.lock().unwrap().push(tx);
        rx
    }
    pub fn read(&self) -> Versioned<Account> {
        self.query(Clone::clone)
    }
    /// Reads a projection and its revision under the same lock.
    pub fn query<T>(&self, read: impl FnOnce(&Account) -> T) -> Versioned<T> {
        let game = self.owner.inner.game.lock().unwrap();
        game.versioned(read(&game.account))
    }
    pub fn handle(&self, command: &Command) -> Result<u64, String> {
        let mut game = self.owner.inner.game.lock().unwrap();
        let ch = match game.handle(command) {
            Ok(ch) => ch,
            Err(e) => {
                self.publish(Event::error(&e));
                return Err(e);
            }
        };
        let revision = ch.revision;
        // Keep publication ordered with commands and with the snapshot it describes.
        for event in crate::events::changes(&game, ch) {
            self.publish(event);
        }
        Ok(revision)
    }
    pub fn import_pulls(&self, uid: u32, pulls: Vec<crate::gacha::Pull>) -> Result<usize, String> {
        let mut game = self.owner.inner.game.lock().unwrap();
        let added = game.import_pulls(uid, pulls, crate::profile::now())?;
        self.publish(Event::new(
            "gacha-changed",
            game.versioned(serde_json::Value::Null),
        ));
        Ok(added)
    }
    /// Writes the player's note of a relic (blank text deletes it) and
    /// publishes `notes-changed` with all notes. Needs a known account.
    pub fn set_relic_note(&self, uid: u32, text: &str) -> Result<(), String> {
        self.edit_notes(|notes, now| notes.set_relic(uid, text, now))
    }
    /// The same for a character (path avatar id).
    pub fn set_character_note(&self, avatar_id: u32, text: &str) -> Result<(), String> {
        self.edit_notes(|notes, now| notes.set_character(avatar_id, text, now))
    }
    fn edit_notes(&self, edit: impl FnOnce(&mut crate::notes::Notes, i64) -> bool) -> Result<(), String> {
        let mut game = self.owner.inner.game.lock().unwrap();
        if game.account.uid.is_none() {
            return Err("аккаунт ещё не определён: войдите в игру с запущенным Nous".into());
        }
        if !edit(&mut game.account.notes, crate::profile::now()) {
            return Ok(());
        }
        game.mark_changed();
        self.publish(Event::new("notes-changed", game.versioned(game.account.notes.clone())));
        Ok(())
    }
    /// Explicit persistence barrier. Errors are returned and also published.
    pub fn flush(&self) -> Result<(), String> {
        self.owner
            .inner
            .flush()
            .inspect_err(|e| self.publish(Event::error(e)))
    }
    /// Adapter diagnostics and lifecycle events share the subscription channel.
    pub fn publish(&self, event: Event) {
        self.owner.inner.publish(event);
    }
}
/// The saver wakes this often for time-driven events and writes the account
/// every SAVE_EVERY_TICKS wakes (1.5 s).
const TICK: Duration = Duration::from_millis(150);
const SAVE_EVERY_TICKS: u32 = 10;

impl Inner {
    fn tick(&self) {
        let mut game = self.game.lock().unwrap();
        if let Some(ch) = game.tick() {
            for event in crate::events::changes(&game, ch) {
                self.publish(event);
            }
        }
    }
    fn publish(&self, event: Event) {
        self.subscribers
            .lock()
            .unwrap()
            .retain(|tx| tx.send(event.clone()).is_ok());
    }
    fn flush(&self) -> Result<(), String> {
        let game = self.game.lock().unwrap();
        let writer = game.writer.clone();
        let _write = writer.lock().unwrap();
        let revision = game.revision();
        let pending = game.pending_snapshot();
        drop(game);
        let result = pending.write();
        // Release the writer before reacquiring state (a switch may be waiting).
        drop(_write);
        if result.is_ok() {
            self.game.lock().unwrap().acknowledge(&pending, revision);
        }
        result
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.stop.send(()).ok();
        if let Some(saver) = self.saver.lock().unwrap().take() {
            let _ = saver.join();
        }
        if let Err(e) = self.inner.flush() {
            self.inner.publish(Event::error(&e));
            log::error!("SDK shutdown: {e}");
        }
    }
}
