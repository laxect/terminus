use tui::{backend::Backend, layout::Rect, Frame};

use crate::message::{Move, Update};

pub(super) struct Input {
    pub label: String,
    pub input: String, // buffer
    pub multi_line: bool,
}

impl Input {
    pub(super) fn new<T: AsRef<str>>(label: T, input: T, multi_line: bool) -> Self {
        Self {
            label: label.as_ref().to_owned(),
            input: input.as_ref().to_owned(),
            multi_line,
        }
    }
}

pub(super) struct Panel {
    edit: bool,
    info: String,
    cursor: usize,
    inputs: Vec<Input>,
}

impl Panel {
    fn new<T: AsRef<str>>(inputs: Vec<Input>, info: T) -> Self {
        assert!(!inputs.is_empty());
        Self {
            inputs,
            cursor: 0,
            edit: false,
            info: info.as_ref().to_owned(),
        }
    }

    fn handle(&mut self, ev: Update) {
        match ev {
            Update::Edit(flag) => {
                self.edit = flag;
            }
            Update::Move(Move::Next) => {
                let last = self.inputs.len() - 1;
                if self.cursor < last {
                    self.cursor += 1;
                } else {
                    self.cursor = 0;
                }
            }
            Update::Move(Move::Prev) => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                } else {
                    self.cursor = self.inputs.len() - 1;
                }
            }
            Update::Input(ch) => {
                self.inputs[self.cursor].input.push(ch);
            }
            _ => {}
        }
    }

    fn layout(&self, area: Rect) -> Vec<Rect> {
        let res = Vec::new();
        let height: usize = self
            .inputs
            .iter()
            .map(|input| if input.multi_line { 2 + 4 } else { 3 })
            .sum();
        res
    }

    pub(super) fn draw<B: Backend>(&self, f: &mut Frame<B>) {}
}
