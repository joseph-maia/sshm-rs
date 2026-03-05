use crossterm::event::{
    Event as CrosstermEvent, EventStream as CrosstermEventStream, KeyEvent, KeyEventKind,
    MouseEvent,
};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Paste(String),
    SshOutput(Vec<u8>),
    SshEof,
    Resize(u16, u16),
    #[allow(dead_code)]
    Tick,
}

pub struct EventLoop {
    rx: mpsc::UnboundedReceiver<Event>,
}

impl EventLoop {
    /// Create a new event loop that merges keyboard and resize events.
    /// SSH output events are injected externally via the returned sender.
    pub fn new() -> (Self, mpsc::UnboundedSender<Event>) {
        let (tx, rx) = mpsc::unbounded_channel();

        let keyboard_tx = tx.clone();
        tokio::spawn(async move {
            let mut reader = CrosstermEventStream::new();
            while let Some(Ok(ev)) = reader.next().await {
                let event = match ev {
                    CrosstermEvent::Key(key) => {
                        // On Windows crossterm fires Press + Release; only handle Press.
                        if key.kind == KeyEventKind::Press {
                            Event::Key(key)
                        } else {
                            continue;
                        }
                    }
                    CrosstermEvent::Mouse(mouse) => Event::Mouse(mouse),
                    CrosstermEvent::Paste(text) => Event::Paste(text),
                    CrosstermEvent::Resize(cols, rows) => Event::Resize(cols, rows),
                    _ => continue,
                };
                if keyboard_tx.send(event).is_err() {
                    break;
                }
            }
        });

        (Self { rx }, tx)
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
