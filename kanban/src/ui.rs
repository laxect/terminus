use crate::message::{self, Update};
use crossbeam_channel::{Receiver, Sender};
use std::io::stdout;
use terminus_types::Node;
use termion::raw::IntoRawMode;
use tui::{
    backend::{Backend, TermionBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};

struct App<'a> {
    list: Vec<Node>,
    info: Spans<'a>,
}

impl Default for App<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl App<'_> {
    fn new() -> Self {
        Self {
            list: Vec::new(),
            info: Self::default_info(),
        }
    }

    fn draw_list<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let main = Block::default().borders(Borders::ALL);
        let text = Text::from("棒球比赛2020年东京奥运会");
        let item = ListItem::new(text);
        let list = List::new(vec![item]).block(main);
        f.render_widget(list, area);
    }

    fn default_info<'a>() -> Spans<'a> {
        Spans::from(vec![
            Span::from("press "),
            Span::styled("q", Style::default().fg(Color::LightRed)),
            Span::from(" to quit"),
        ])
    }

    fn draw_info<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let infomation_block = Block::default().borders(Borders::ALL);
        let info = Paragraph::new(self.info.clone())
            .block(infomation_block)
            .wrap(Wrap { trim: true });
        f.render_widget(info, area);
    }

    fn set_info(&mut self, msg: String) {
        let info = Spans::from(vec![Span::from(msg)]);
        self.info = info;
    }

    fn set_info_err(&mut self, err: String) {
        let info = Spans::from(vec![Span::styled(err, Style::default().fg(Color::LightRed))]);
        self.info = info;
    }

    fn draw<B: Backend>(&self, f: &mut Frame<B>) {
        // get layout
        let size = f.size();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Max(size.height - 3), Constraint::Max(3)].as_ref())
            .split(f.size());
        // draw
        self.draw_list(f, chunks[0]);
        self.draw_info(f, chunks[1]);
    }
}

pub(crate) fn run(s: Sender<message::Request>, r: Receiver<message::Update>) -> anyhow::Result<()> {
    print!("{}", termion::clear::All);
    let stdout = stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    // set up app
    let mut app = App::default();
    loop {
        terminal.draw(|f| app.draw(f))?;
        let event = r.recv()?;
        match event {
            Update::Quit => {
                break;
            }
            Update::Err(e) => {
                app.set_info_err(e.to_string());
            }
            _ => {
                todo!()
            }
        }
    }
    Ok(())
}
