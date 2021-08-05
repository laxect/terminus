use super::split::UnicodeSplit;
use crate::message::{Move, Update};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone)]
pub(crate) struct Input {
    pub label: String,
    pub input: String, // buffer
    pub multi_line: bool,
}

impl Input {
    pub(crate) fn new<T: AsRef<str>>(label: T, input: T, multi_line: bool) -> Self {
        Self {
            label: label.as_ref().to_owned(),
            input: input.as_ref().to_owned(),
            multi_line,
        }
    }

    fn draw<B: Backend>(&self, f: &mut Frame<B>, area: Rect, selected: bool, edit: bool) {
        let style = Style {
            fg: if selected { Some(Color::LightYellow) } else { None },
            ..Default::default()
        };
        let block = Block::default()
            .border_style(style)
            .borders(Borders::all())
            .title(Span::raw(&self.label));
        let width = block.inner(area).width as usize;
        if self.multi_line {
            let split = self.input.split('\n').map(|str| str.unicode_split(width)).flatten();
            let count = std::cmp::max(split.clone().count(), 3);
            let mut take: Vec<&str> = split.skip(count - 3).collect();
            if take.last().map(|str| str.width_cjk()) == Some(width) || self.input.ends_with('\n') {
                take.push("");
                if take.len() > 3 {
                    take.remove(0);
                }
            }
            if edit {
                let y = area.y + std::cmp::max(take.len() as u16, 1);
                let x = area.x + 1 + take.last().map(|str| str.width_cjk() as u16).unwrap_or(0);
                f.set_cursor(x, y)
            }
            let spans: Vec<Spans> = take.into_iter().map(Spans::from).collect();
            let text = Paragraph::new(spans).block(block);
            f.render_widget(text, area);
        } else {
            let mut width = width - 1;
            if self.input.len() < width {
                width = self.input.len();
            }
            let show = &self.input[self.input.len() - width..];
            let input = Span::styled(show, style);
            let text = Paragraph::new(input).block(block);
            f.render_widget(text, area);
            if edit {
                let x = area.x + 1 + show.width_cjk() as u16;
                let y = area.y + 1;
                f.set_cursor(x, y)
            }
        }
    }
}

pub(super) enum PanelMode {
    Panel,
    Info,
}

pub(super) struct Panel {
    edit: bool,
    info: String,
    scroll: u16,
    cursor: usize,
    mode: PanelMode,
    inputs: Vec<Input>,
}

impl Panel {
    pub(super) fn new<T: AsRef<str>>(inputs: Vec<Input>, info: T, mode: PanelMode) -> Self {
        assert!(!inputs.is_empty() || !matches!(mode, PanelMode::Panel));
        Self {
            mode,
            inputs,
            cursor: 0,
            scroll: 0,
            edit: false,
            info: info.as_ref().to_owned(),
        }
    }

    pub(super) fn handle(&mut self, ev: Update) {
        match ev {
            Update::Edit(flag) => {
                self.edit = flag;
            }
            Update::Move(Move::Next) => match self.mode {
                PanelMode::Panel => {
                    let last = self.inputs.len() - 1;
                    if self.cursor < last {
                        self.cursor += 1;
                    } else {
                        self.cursor = 0;
                    }
                }
                PanelMode::Info => {
                    let text = Text::from(self.info.as_str());
                    let h = text.height() as u16;
                    if self.scroll < h.saturating_sub(5) {
                        self.scroll += 1;
                    }
                }
            },
            Update::Move(Move::Prev) => match self.mode {
                PanelMode::Panel => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                    } else {
                        self.cursor = self.inputs.len() - 1;
                    }
                }
                PanelMode::Info => {
                    self.scroll = self.scroll.saturating_sub(1);
                }
            },
            Update::Input('\n') => {
                let input = &mut self.inputs[self.cursor];
                if !input.input.is_empty() && input.multi_line {
                    input.input.push('\n');
                }
            }
            Update::Input(ch) => {
                self.inputs[self.cursor].input.push(ch);
            }
            Update::DeleteChar => {
                self.inputs[self.cursor].input.pop();
            }
            _ => unreachable!(),
        }
    }

    const MULTI_LINE_HEIGHT: u16 = 4 + 2;
    fn panel_layout(&self, area: Rect) -> Vec<Rect> {
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(20),
                    Constraint::Percentage(60),
                    Constraint::Percentage(20),
                ]
                .as_ref(),
            )
            .split(area);
        let height: u16 = self
            .inputs
            .iter()
            .map(|input| if input.multi_line { Self::MULTI_LINE_HEIGHT } else { 3 })
            .sum();
        // margin 1, info 3.
        let height = height + 2 + 3;
        // if height is higher than area, means you should use a bigger terminal.
        let spaces = area.height.checked_sub(height).unwrap_or_default();
        let top = spaces / 2;
        let mut chunks = vec![Constraint::Max(top)];
        for input in self.inputs.iter() {
            let block = Constraint::Max(if input.multi_line { Self::MULTI_LINE_HEIGHT } else { 3 });
            chunks.push(block);
        }
        chunks.push(Constraint::Max(5)); // info block
        chunks.push(Constraint::Max(1)); // space
        let mut blocks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(chunks)
            .split(horizontal[1]);
        // remove space
        blocks.remove(0);
        blocks.remove(blocks.len() - 1);
        blocks
    }

    fn draw_panel<B: Backend>(&self, f: &mut Frame<B>) {
        let terminal = f.size();
        let mut layout = self.panel_layout(terminal);
        let info = layout.pop().unwrap();
        // should always be same length
        for (ind, (input, area)) in self.inputs.iter().zip(layout.into_iter()).enumerate() {
            input.draw(f, area, ind == self.cursor, ind == self.cursor && self.edit);
        }
        // draw Info
        let text = Paragraph::new(self.info.as_str())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::LightBlue))
            .block(Block::default().borders(Borders::all()));
        f.render_widget(text, info);
    }

    fn draw_info<B: Backend>(&self, f: &mut Frame<B>) {
        let terminal = f.size();
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(15),
                    Constraint::Percentage(70),
                    Constraint::Percentage(15),
                ]
                .as_ref(),
            )
            .split(terminal);
        let area = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ])
            .split(horizontal[1]);
        let block = Block::default().borders(Borders::all()).title("info");
        let text = Paragraph::new(self.info.as_str()).block(block).scroll((self.scroll, 0));
        f.render_widget(text, area[1]);
    }

    pub(super) fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        match self.mode {
            PanelMode::Panel => self.draw_panel(f),
            PanelMode::Info => self.draw_info(f),
        }
    }

    /// Get a reference to the panel's inputs.
    pub(super) fn inputs(&self) -> &[Input] {
        self.inputs.as_slice()
    }
}
