use crossterm::event::Event as CrosstermEvent;
use futures::{FutureExt, StreamExt};
use std::time::Duration;
use tokio::sync::mpsc;

/// The frequency at which tick events are emitted
const TICK_FPS: f64 = 30.0;

/// Events that can occur in the application
#[derive(Debug)]
pub enum Event {
    /// Terminal events (key presses, mouse events, etc.)
    Terminal(CrosstermEvent),
    /// Regular tick for animations
    Tick,
    /// Application specific events
    App(AppEvent),
}

/// Application specific events
#[derive(Debug)]
pub enum AppEvent {
    /// Submit the current input
    Submit(String),
    /// Received response from LLM
    LLMResponse(String),
    /// Error from LLM
    LLMError(String),
    /// Quit the application
    Quit,
}

/// Event handler that manages the event stream
pub struct EventHandler {
    /// Event sender
    sender: mpsc::UnboundedSender<Event>,
    /// Event receiver
    receiver: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let event_sender = sender.clone();

        // Spawn the event handling task
        tokio::spawn(async move {
            let tick_rate = Duration::from_secs_f64(1.0 / TICK_FPS);
            let mut reader = crossterm::event::EventStream::new();
            let mut tick = tokio::time::interval(tick_rate);

            loop {
                let tick_delay = tick.tick();
                let crossterm_event = reader.next().fuse();

                tokio::select! {
                    _ = event_sender.closed() => {
                        break;
                    }
                    _ = tick_delay => {
                        let _ = event_sender.send(Event::Tick);
                    }
                    Some(Ok(evt)) = crossterm_event => {
                        let _ = event_sender.send(Event::Terminal(evt));
                    }
                }
            }
        });

        Self { sender, receiver }
    }

    /// Get the event sender
    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.sender.clone()
    }

    /// Get the next event
    pub async fn next(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }
}
