//! One worker thread owns the pasteboard and the captured AX element.
//!
//! Neither is `Send`, and both must stay out of the frame loop, so the app sends commands here
//! and gets plain data back through a callback. The capture is kept only until the flow ends.

use crate::apply;
use crate::capture::{self, Capture};
use crate::flow::Captured;
use crate::mac::pasteboard::Board;
use crate::placement::Rect;
use rune::machine::Attempt;
use std::sync::mpsc::{Sender, channel};
use std::thread;

pub enum Command {
    Capture(Attempt),
    Replace {
        attempt: Attempt,
        text: String,
    },
    /// The flow is back to Idle: drop the captured element and selection.
    Forget,
}

#[derive(Debug)]
pub enum Reply {
    Captured { attempt: Attempt, result: Result<Captured, String>, caret: Option<Rect> },
    Applied { attempt: Attempt, result: Result<(), String> },
}

pub struct Native {
    commands: Sender<Command>,
}

impl Native {
    /// `on_reply` runs on the worker thread; it should hand the reply to the app and wake it.
    pub fn start(on_reply: impl Fn(Reply) + Send + 'static) -> std::io::Result<Self> {
        let (commands, receive) = channel::<Command>();
        thread::Builder::new().name("retone-native".into()).spawn(move || {
            let board = Board::general();
            let mut current: Option<Capture> = None;
            for command in receive {
                match command {
                    Command::Capture(attempt) => {
                        current = None;
                        let reply = match capture::capture(&board) {
                            Ok(capture) => {
                                let caret = capture.target.as_ref().and_then(|target| target.caret_bounds());
                                let captured = capture.captured.clone();
                                current = Some(capture);
                                Reply::Captured { attempt, result: Ok(captured), caret }
                            }
                            Err(error) => Reply::Captured { attempt, result: Err(error), caret: None },
                        };
                        on_reply(reply);
                    }
                    Command::Replace { attempt, text } => {
                        let result = match current.as_ref() {
                            Some(capture) => apply::replace(capture, &text, &board),
                            None => Err(t!("apply.nothing_captured").into()),
                        };
                        on_reply(Reply::Applied { attempt, result });
                    }
                    Command::Forget => current = None,
                }
            }
        })?;
        Ok(Self { commands })
    }

    pub fn send(&self, command: Command) {
        // The worker only stops when the app exits; a failed send then changes nothing.
        self.commands.send(command).ok();
    }
}
