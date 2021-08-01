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

fn handle_input(_s: &Sender<Update>) -> anyhow::Result<()> {
    Ok(())
}

pub(crate) fn set_input_mode(on: bool) {
    INPUT_MODE.store(on, Ordering::Release);
}

pub(crate) fn handle(s: Sender<Update>) -> anyhow::Result<()> {
    let stdin = stdin();
    for c in stdin.events() {
        if INPUT_MODE.load(Ordering::Acquire) {
            handle_input(&s)?;
            continue;
        }
        let c = c?;
        match c {
            Event::Key(Key::Char('q')) => {
                s.send(Update::Quit)?;
                return Ok(());
            }
            Event::Key(Key::Char('j')) => {
                s.send(Update::Next)?;
            }
            Event::Key(Key::Char('k')) => {
                s.send(Update::Prev)?;
            }
            Event::Key(Key::Char('h')) => {
                s.send(Update::Parent)?;
            }
            Event::Key(Key::Char('l')) => {
                s.send(Update::Child)?;
            }
            _ => {
                log::trace!("{:?} received.", c);
            }
        }
    }
    Ok(())
}
