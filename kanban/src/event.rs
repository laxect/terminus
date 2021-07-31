use crate::message::Update;
use crossbeam_channel::Sender;
use std::{
    io::stdin,
    sync::atomic::{AtomicBool, Ordering},
};
use termion::{
    event::{Event, Key},
    input::TermRead,
};

static INPUT_MODE: AtomicBool = AtomicBool::new(false);

pub(crate) fn handle(s: Sender<Update>) -> anyhow::Result<()> {
    let stdin = stdin();
    for c in stdin.events() {
        if INPUT_MODE.load(Ordering::SeqCst) {
            continue;
        }
        let c = c?;
        match c {
            Event::Key(Key::Char('q')) => {
                s.send(Update::Quit)?;
                return Ok(());
            }
            _ => {
                log::trace!("{:?} received.", c);
            }
        }
    }
    Ok(())
}
