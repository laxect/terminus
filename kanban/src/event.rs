use crate::message::Update;
use crossbeam_channel::Sender;
use std::io::stdin;
use termion::{
    event::{Event, Key},
    input::TermRead,
};

pub(crate) fn handle(s: Sender<Update>) -> anyhow::Result<()> {
    let stdin = stdin();
    for c in stdin.events() {
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
