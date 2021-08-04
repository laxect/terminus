use crate::message::{EditPanel, Move, OpenPanel, PanelAction, Update};
use crossbeam_channel::Sender;
use crossbeam_utils::atomic::AtomicCell;
use std::io::stdin;
use termion::{
    event::{Event, Key},
    input::TermRead,
};

static MODE: AtomicCell<Mode> = AtomicCell::new(Mode::Normal);

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Normal = 0,
    Panel = 1,
    Input = 2,
}

fn set_mode(on: Mode) {
    MODE.store(on);
}

fn handle_input(s: &Sender<Update>, c: Event) -> anyhow::Result<()> {
    match c {
        Event::Key(Key::Esc) => {
            s.send(Update::Edit(false))?;
            set_mode(Mode::Panel);
        }
        Event::Key(Key::Char(ch)) => {
            s.send(Update::Input(ch))?;
        }
        Event::Key(Key::Backspace) => {
            s.send(Update::DeleteChar)?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_panel(s: &Sender<Update>, c: Event) -> anyhow::Result<()> {
    match c {
        Event::Key(Key::Char('j') | Key::Down) => {
            s.send(Update::Move(Move::Next))?;
        }
        Event::Key(Key::Char('k') | Key::Up) => {
            s.send(Update::Move(Move::Prev))?;
        }
        Event::Key(Key::Char('i' | 'o')) => {
            s.send(Update::Edit(true))?;
            set_mode(Mode::Input);
        }
        Event::Key(Key::Char('q') | Key::Esc) => {
            s.send(Update::PanelAction(PanelAction::Cancel))?;
            set_mode(Mode::Normal);
        }
        Event::Key(Key::Char('\n')) => {
            s.send(Update::PanelAction(PanelAction::Confirm))?;
            set_mode(Mode::Normal);
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn handle(s: Sender<Update>) -> anyhow::Result<()> {
    let stdin = stdin();
    for c in stdin.events() {
        let c = c?;
        let mode: Mode = MODE.load();
        if mode == Mode::Input {
            handle_input(&s, c)?;
            continue;
        } else if mode == Mode::Panel {
            handle_panel(&s, c)?;
            continue;
        }
        match c {
            Event::Key(Key::Char('q')) => {
                s.send(Update::Quit)?;
                return Ok(());
            }
            Event::Key(Key::Char('j') | Key::Down) => {
                s.send(Update::Move(Move::Next))?;
            }
            Event::Key(Key::Char('k') | Key::Up) => {
                s.send(Update::Move(Move::Prev))?;
            }
            Event::Key(Key::Char('h') | Key::Left) => {
                s.send(Update::Move(Move::Parent))?;
            }
            Event::Key(Key::Char('l') | Key::Right) => {
                s.send(Update::Move(Move::Child))?;
            }
            Event::Key(Key::Char('g')) => {
                s.send(Update::Move(Move::Top))?;
            }
            Event::Key(Key::Char('G')) => {
                s.send(Update::Move(Move::Bottom))?;
            }
            Event::Key(Key::Char('s')) => {
                s.send(Update::OpenPanel(OpenPanel::Setting))?;
                set_mode(Mode::Panel);
            }
            Event::Key(Key::Char('p' | 'n')) => {
                s.send(Update::OpenPanel(OpenPanel::EditPanel(EditPanel::Post)))?;
                set_mode(Mode::Panel);
            }
            Event::Key(Key::Char('r')) => {
                s.send(Update::OpenPanel(OpenPanel::EditPanel(EditPanel::Reply)))?;
                set_mode(Mode::Panel);
            }
            _ => {
                log::trace!("{:?} received.", c);
            }
        }
    }
    Ok(())
}
