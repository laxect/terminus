use crate::{
    config::Config,
    message::{EditPanel, Move, OpenPanel, PanelAction, Request, Update},
    store::Store,
    ui::panel::PanelMode,
};
use chrono::Local;
use crossbeam_channel::{Receiver, Sender};
use panel::Panel;
use split::UnicodeSplit;
use std::{
    io::stdout,
    mem::swap,
    sync::{Arc, Mutex},
};
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

mod edit_panel;
mod help;
pub(crate) mod panel;
mod split;

#[derive(Debug)]
enum State {
    Help,
    // list mode
    Root,
    Node(NodeId),
    Setting,
    // panel
    Post,
    Reply(Vec<u8>),
    Update(Node),
    Delete(Node),
}

struct App<'a> {
    list: Vec<Node>,
    info: Spans<'a>,
    state: Vec<State>,
    store: Store,
    panel: Option<Panel>,
    list_state: ListState,
    cur_stack: Vec<ListState>,
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
            list: Vec::new(),
            info: Self::default_info(),
            state: vec![State::Root],
            store: Store::new()?,
            panel: None,
            cur_stack: Vec::new(),
            list_state: ListState::default(),
        })
    }

    fn draw_title<'a>(&self, title: String, width: usize, mut space: usize) -> Text<'a> {
        space = std::cmp::min(space, BLANK.len());
        let color = space as u8;
        let color = if let State::Root = self.state() {
            Color::Rgb(50, 50, 255 - color)
        } else {
            Color::Rgb(color, color, color)
        };
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
        for line in title {
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
        let split = content.split('\n').map(|str| str.unicode_split(width)).flatten();
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
        let mut text = self.draw_title(node.title, width, spaces);
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
            let blank_len = std::cmp::min(blank_len, BLANK.len());
            author_line.insert(0, Span::from(&BLANK[0..blank_len]));
        }
        // content part
        let max_content_height = if let State::Root = self.state.last().unwrap() {
            Some(3)
        } else {
            None
        };
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
        if let Some(ref panel) = self.panel {
            panel.draw(f);
            return;
        }
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
        match self.state() {
            State::Root => {
                self.list = self.store.list_root()?;
            }
            State::Node(node) => {
                self.list = self.store.list(node)?;
            }
            _ => {
                return Ok(());
            }
        }
        let now = self.list_state.selected();
        if let Some(now) = now {
            if now >= self.list.len() {
                let next = self.list.len().checked_sub(1);
                self.list_state.select(next);
            }
        } else {
            self.list_state
                .select(if self.list.is_empty() { None } else { Some(0) });
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

    fn top(&mut self) {
        self.list_state
            .select(if self.list.is_empty() { None } else { Some(0) });
    }

    fn bottom(&mut self) {
        self.list_state.select(if self.list.is_empty() {
            None
        } else {
            Some(self.list.len() - 1)
        });
    }

    fn go_down(&mut self, s: &Sender<Request>) {
        let node_id = if let Some(node) = self.selected() {
            node.id.to_owned()
        } else {
            return;
        };
        // don't go on same node
        if let State::Node(now) = self.state() {
            if now == &node_id {
                return;
            }
        }
        let mut new_list_state = ListState::default();
        new_list_state.select(Some(0));
        swap(&mut self.list_state, &mut new_list_state);
        self.cur_stack.push(new_list_state);
        self.state.push(State::Node(node_id.clone()));
        let req = Request::List(node_id);
        req.send(s).unwrap();
    }

    fn go_above(&mut self, s: &Sender<Request>) {
        let node_id = if let State::Node(ref id) = self.state() {
            id.to_owned()
        } else {
            return;
        };
        // check length
        let length = node_id.len();
        let prev_cur = self.cur_stack.pop().unwrap_or_default();
        self.list_state = prev_cur;
        self.state.pop();
        let req = if length <= 16 {
            Request::ListRoot
        } else {
            Request::List(node_id)
        };
        req.send(s).unwrap();
    }

    /// Get a reference to the app's state.
    fn state(&self) -> &State {
        self.state.last().unwrap()
    }

    pub(crate) fn selected(&self) -> Option<&Node> {
        if let Some(now) = self.list_state.selected() {
            Some(&self.list[now])
        } else {
            None
        }
    }
}

const ROOT_ID: &Vec<u8> = &vec![];
pub(crate) fn run(s: Sender<Request>, r: Receiver<Update>, config: Arc<Mutex<Config>>) -> anyhow::Result<()> {
    let stdout = stdout().into_raw_mode()?;
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    // set up app
    let mut app = App::default();
    let req = Request::ListRoot;
    req.send(&s).expect("inital list failed.");
    loop {
        terminal.draw(|f| app.draw(f))?;
        let event = r.recv()?;
        if let Some(panel) = app.panel.as_mut() {
            match event {
                Update::PanelAction(PanelAction::Confirm) => {
                    let inputs = panel.inputs();
                    match app.state.pop().unwrap() {
                        State::Setting => {
                            let mut config = config.lock().unwrap();
                            config.set_val_from_inputs(inputs);
                            config.save_to_file().ok();
                            s.send(Request::Relink)?;
                            let req = Request::ListRoot;
                            req.send(&s)?;
                            app.store.clear();
                            app.refesh_list()?;
                        }
                        State::Post => {
                            // equal to app.state() but can not do that because of ref rule
                            let node_id = if let State::Node(node_id) = app.state.last().unwrap() {
                                node_id
                            } else {
                                ROOT_ID
                            };
                            edit_panel::post_node(&s, node_id, inputs, config.lock().unwrap().gen_author()).unwrap();
                        }
                        State::Reply(ref node_id) => {
                            edit_panel::post_node(&s, node_id, inputs, config.lock().unwrap().gen_author()).unwrap();
                        }
                        State::Update(node) => {
                            edit_panel::update_node(&s, node, inputs, config.lock().unwrap().gen_author()).unwrap();
                        }
                        State::Delete(node) => {
                            edit_panel::delete_node(&s, node, config.lock().unwrap().gen_author()).unwrap();
                        }
                        State::Help => {}
                        _ => unreachable!(),
                    }
                    app.panel = None;
                }
                Update::PanelAction(PanelAction::Cancel) => {
                    app.state.pop();
                    app.panel = None;
                }
                _ => {
                    panel.handle(event);
                }
            }
            continue;
        }
        match event {
            Update::Err(e) => {
                app.set_info_err(e.to_string());
            }
            Update::Nodes(nodes) => {
                for node in nodes {
                    app.store.insert(node)?;
                }
                app.refesh_list()?;
            }
            Update::DeleteNode(node) => {
                app.store.delete(&node).ok();
                app.refesh_list()?;
            }
            Update::Quit => {
                // press 'q'
                let req = Request::Shutdown;
                // result is not important.
                req.send(&s).ok();
                break;
            }
            Update::Move(Move::Next) => {
                app.next();
            }
            Update::Move(Move::Prev) => {
                app.prev();
            }
            Update::Move(Move::Child) => {
                app.go_down(&s);
                app.refesh_list()?;
            }
            Update::Move(Move::Parent) => {
                app.go_above(&s);
                app.refesh_list()?;
            }
            Update::Move(Move::Top) => {
                app.top();
            }
            Update::Move(Move::Bottom) => {
                app.bottom();
            }
            Update::OpenPanel(OpenPanel::Setting) => {
                let inputs = config.lock().unwrap().gen_inputs();
                let panel = Panel::new(
                    inputs,
                    "press i for input, ESC for quit, Return for confirm.",
                    PanelMode::Panel,
                );
                app.panel = Some(panel);
                app.state.push(State::Setting);
            }
            Update::OpenPanel(OpenPanel::EditPanel(EditPanel::Post)) => {
                app.panel = Some(edit_panel::post_panel(None));
                app.state.push(State::Post);
            }
            Update::OpenPanel(OpenPanel::EditPanel(EditPanel::Update)) => {
                if let Some(node) = app.selected() {
                    let node = node.clone();
                    app.panel = Some(edit_panel::update_panel(None, &node));
                    app.state.push(State::Update(node));
                }
            }
            Update::OpenPanel(OpenPanel::EditPanel(EditPanel::Reply)) => {
                let node_id = if let Some(node) = app.selected() {
                    node.id.to_owned()
                } else {
                    continue;
                };
                app.panel = Some(edit_panel::post_panel(Some("reply to node")));
                app.state.push(State::Reply(node_id));
            }
            Update::OpenPanel(OpenPanel::Help) => {
                app.panel = Some(help::help_panel());
                app.state.push(State::Help);
            }
            Update::OpenPanel(OpenPanel::Delete) => {
                if let Some(node) = app.selected() {
                    let node = node.clone();
                    app.panel = Some(edit_panel::delete_confirm(&node.title));
                    app.state.push(State::Delete(node));
                }
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}
