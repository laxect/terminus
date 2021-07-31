use crate::{
    message::{self, Request, Update},
    store::Store,
};
use chrono::Local;
use crossbeam_channel::{Receiver, Sender};
use std::io::stdout;
use terminus_types::{Node, NodeId};
use termion::{raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::{Backend, TermionBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};

enum State {
    Root,
    Node(NodeId),
}

#[derive(Default)]
struct AppDiff {
    list: bool,
    info: bool,
}

struct App<'a> {
    state: State,
    diff: AppDiff,
    list: Vec<Node>,
    store: Store,
    info: Spans<'a>,
}

impl Default for App<'_> {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl App<'_> {
    fn new() -> anyhow::Result<Self> {
        Ok(Self {
            state: State::Root,
            list: Vec::new(),
            store: Store::new()?,
            info: Self::default_info(),
            diff: AppDiff { info: true, list: true },
        })
    }

    fn draw_title<'a>(mut title: String, width: usize) -> Text<'a> {
        // start from '# '.
        let width = width - 2;
        title.insert_str(0, "# ");
        Text::styled(
            title,
            Style::default().add_modifier(Modifier::BOLD).fg(Color::Rgb(0, 0, 0)),
        )
    }

    fn draw_node<'a>(mut node: Node, width: usize) -> ListItem<'a> {
        node.author.mask();
        // title
        let mut text = Self::draw_title(node.title, width);
        // author part
        let edited = if node.edited {
            Color::LightCyan
        } else {
            Color::LightGreen
        };
        let edited = Span::styled(if node.edited { "edited " } else { "" }, Style::default().fg(edited));
        let id = Span::styled(node.author.encode_pass(6), Style::default().fg(Color::LightBlue));
        let author = Span::styled(node.author.name, Style::default().fg(Color::LightBlue));
        let splt_sym = Span::from("#");
        let at_sym = Span::styled(" @ ", Style::default().add_modifier(Modifier::BOLD).fg(Color::LightRed));
        let publish_time = Span::styled(
            node.publish_time
                .with_timezone(&Local)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
            Style::default().add_modifier(Modifier::ITALIC),
        );
        let mut author_line = vec![edited, author, splt_sym, id, at_sym, publish_time];
        let line_width: usize = author_line.iter().map(|sp| sp.width()).sum();
        let blank_len = width - line_width;
        if blank_len > 0 {
            let blank = vec![' ' as u8; blank_len];
            let blank = String::from_utf8(blank).unwrap();
            author_line.insert(0, Span::from(blank));
        }
        // content part
        let content = Spans::from(node.content);
        text.extend(Text::from(vec![content, Spans::from(author_line)]));
        ListItem::new(text).style(Style::default())
    }

    fn draw_list<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        if !self.diff.list {
            return;
        }
        self.diff.list = false;
        // node
        let main = Block::default().borders(Borders::ALL);
        let inner_width = main.inner(area).width;
        let mut list = Vec::new();
        for node in &self.list {
            list.push(Self::draw_node(node.clone(), inner_width as usize));
        }
        // List
        let list = List::new(list).block(main);
        f.render_widget(list, area);
    }

    fn default_info<'a>() -> Spans<'a> {
        Spans::from(vec![
            Span::from("press "),
            Span::styled("q", Style::default().fg(Color::LightRed)),
            Span::from(" to quit"),
        ])
    }

    fn draw_info<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        if !self.diff.info {
            return;
        }
        self.diff.info = false;
        let infomation_block = Block::default().borders(Borders::ALL);
        let info = Paragraph::new(self.info.clone())
            .block(infomation_block)
            .wrap(Wrap { trim: true });
        f.render_widget(info, area);
    }

    fn set_info(&mut self, msg: String) {
        let info = Spans::from(vec![Span::from(msg)]);
        self.info = info;
        self.diff.info = true;
    }

    fn set_info_err(&mut self, err: String) {
        let info = Spans::from(vec![Span::styled(err, Style::default().fg(Color::LightRed))]);
        self.info = info;
        self.diff.info = true;
    }

    fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
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

    fn refesh_list(&mut self) -> anyhow::Result<()> {
        match &self.state {
            State::Root => {
                self.list = self.store.list_root()?;
            }
            State::Node(node) => {
                self.list = self.store.list(node)?;
            }
        }
        self.diff.list = true;
        Ok(())
    }
}

pub(crate) fn run(s: Sender<message::Request>, r: Receiver<message::Update>) -> anyhow::Result<()> {
    let stdout = stdout().into_raw_mode()?;
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    // set up app
    let mut app = App::default();
    loop {
        terminal.draw(|f| app.draw(f))?;
        let event = r.recv()?;
        match event {
            Update::Quit => {
                let req = Request::Shutdown;
                // result is not important.
                req.send(&s).ok();
                break;
            }
            Update::Err(e) => {
                app.set_info_err(e.to_string());
            }
            Update::Nodes(nodes) => {
                for node in nodes {
                    app.store.insert(node)?;
                }
                app.refesh_list()?;
                app.set_info("update received".to_string());
            }
            Update::DeleteNode(node) => {
                app.store.delete(node).ok();
                app.refesh_list()?;
                app.set_info("delete received".to_string());
            }
        }
    }
    Ok(())
}
