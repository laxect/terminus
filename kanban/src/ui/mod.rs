use crate::{
    message::{Request, Update},
    store::Store,
};
use chrono::Local;
use crossbeam_channel::{Receiver, Sender};
use split::UnicodeSplit;
use std::io::stdout;
use terminus_types::{Node, NodeId};
use termion::{raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::{Backend, TermionBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

mod split;

#[derive(Debug)]
enum State {
    Root,
    Node(NodeId),
}

struct App<'a> {
    state: State,
    list: Vec<Node>,
    list_state: ListState,
    cur_stack: Vec<usize>,
    store: Store,
    info: Spans<'a>,
}

impl Default for App<'_> {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

const BLANK: &str = "                                                     ";
impl App<'_> {
    fn new() -> anyhow::Result<Self> {
        Ok(Self {
            state: State::Root,
            list: Vec::new(),
            store: Store::new()?,
            cur_stack: Vec::new(),
            info: Self::default_info(),
            list_state: ListState::default(),
        })
    }

    fn draw_title<'a>(title: String, width: usize, mut space: usize) -> Text<'a> {
        space = std::cmp::min(space, BLANK.len());
        let color = space as u8;
        let color = Color::Rgb(color, color, color);
        // start from '# '.
        let width = width - 2 - space;
        let mut title = title.unicode_split(width);
        let first = title.next().unwrap_or_default().to_owned();
        let first = Spans::from(vec![
            Span::from(&BLANK[0..space]),
            Span::styled("# ", Style::default().add_modifier(Modifier::BOLD).fg(color)),
            Span::styled(first, Style::default().add_modifier(Modifier::BOLD).fg(color)),
        ]);
        let mut lines = vec![first];
        while let Some(line) = title.next() {
            let line = Spans::from(vec![
                Span::from(&BLANK[0..space]),
                Span::styled("  ", Style::default().add_modifier(Modifier::BOLD).fg(color)),
                Span::styled(
                    line.trim_start().to_owned(),
                    Style::default().add_modifier(Modifier::BOLD).fg(color),
                ),
            ]);
            lines.push(line);
        }
        Text::from(lines)
    }

    fn draw_content<'a>(content: String, width: usize, mut space: usize, max_height: Option<usize>) -> Text<'a> {
        space = std::cmp::min(space, BLANK.len());
        let width = width - space;
        let blank = ["\n", &BLANK[..space]].concat();
        let split = content.unicode_split(width);
        let mut content = if let Some(max_height) = max_height {
            let mut content = split
                .take(max_height)
                .map(|str| str.trim_start())
                .collect::<Vec<&str>>()
                .join(&blank);
            if content.width_cjk() > width * 2 + (width / 2) {
                for _i in 0..4 {
                    content.pop();
                }
                content.push_str("……");
            }
            content
        } else {
            split.map(|str| str.trim_start()).collect::<Vec<&str>>().join(&blank)
        };
        content.insert_str(0, &BLANK[0..space]);
        Text::from(content)
    }

    fn draw_node<'a>(&self, mut node: Node, width: usize) -> ListItem<'a> {
        node.author.mask();
        let level = node.id.len() / 16;
        let spaces = level.saturating_sub(1) * 2;
        // title
        let mut text = Self::draw_title(node.title, width, spaces);
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
        let max_content_height = if let State::Root = self.state { Some(3) } else { None };
        text.extend(Self::draw_content(node.content, width, spaces, max_content_height));
        text.extend(Text::from(Spans::from(author_line)));
        ListItem::new(text).style(Style::default())
    }

    fn draw_list<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        // node
        let main = Block::default().borders(Borders::ALL);
        let inner_width = main.inner(area).width;
        let mut list = Vec::new();
        for node in &self.list {
            list.push(self.draw_node(node.clone(), inner_width as usize));
        }
        // List
        let mut now = self.list_state.selected();
        if let Some(now) = now.as_mut() {
            if *now >= list.len() {
                let next = list.len().checked_sub(1);
                self.list_state.select(next);
            }
        } else {
            self.list_state.select(if list.is_empty() { None } else { Some(0) });
        }
        let list = List::new(list)
            .block(main)
            .highlight_style(Style::default().bg(Color::Rgb(235, 235, 235)));
        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn default_info<'a>() -> Spans<'a> {
        Spans::from(vec![
            Span::from("press "),
            Span::styled("q", Style::default().fg(Color::LightRed)),
            Span::from(" to quit"),
        ])
    }

    fn draw_info<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
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
        Ok(())
    }

    fn next(&mut self) {
        let max = self.list.len();
        if let Some(mut now) = self.list_state.selected() {
            now += 1;
            if now >= max {
                now = 0;
            }
            self.list_state.select(Some(now));
        }
    }

    fn prev(&mut self) {
        let max = self.list.len();
        if let Some(mut now) = self.list_state.selected() {
            if now == 0 {
                now = max;
            }
            now -= 1;
            self.list_state.select(Some(now));
        }
    }

    fn go_down(&mut self, s: &Sender<Request>) {
        let now = if let Some(now) = self.list_state.selected() {
            now
        } else {
            return;
        };
        self.cur_stack.push(now);
        self.list_state = ListState::default();
        self.list_state.select(Some(0));
        let node_id = self.list[now].id.to_owned();
        self.state = State::Node(node_id.clone());
        let req = Request::List(node_id);
        req.send(s).unwrap();
    }

    fn go_above(&mut self, s: &Sender<Request>) {
        let node_id = if let State::Node(ref id) = self.state {
            id.to_owned()
        } else {
            return;
        };
        // check length
        let length = node_id.len();
        let prev_cur = self.cur_stack.pop();
        self.list_state = ListState::default();
        self.list_state.select(Some(0));
        self.list_state.select(prev_cur);
        let req = if length <= 16 {
            self.state = State::Root;
            Request::ListRoot
        } else {
            self.state = State::Node(node_id[..length - 16].to_owned());
            Request::List(node_id)
        };
        req.send(s).unwrap();
    }
}

pub(crate) fn run(s: Sender<Request>, r: Receiver<Update>) -> anyhow::Result<()> {
    let stdout = stdout().into_raw_mode()?;
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    // set up app
    let mut app = App::default();
    loop {
        app.set_info(format!("{:?}", app.state));
        terminal.draw(|f| app.draw(f))?;
        let event = r.recv()?;
        match event {
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
            Update::Quit => {
                // press 'q'
                let req = Request::Shutdown;
                // result is not important.
                req.send(&s).ok();
                break;
            }
            Update::Next => {
                app.next();
            }
            Update::Prev => {
                app.prev();
            }
            Update::Child => {
                app.go_down(&s);
                app.refesh_list()?;
            }
            Update::Parent => {
                app.go_above(&s);
                app.refesh_list()?;
            }
        }
    }
    Ok(())
}
