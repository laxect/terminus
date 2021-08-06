use super::panel::{Panel, PanelMode};
use crate::{message::Request, ui::panel::Input};
use crossbeam_channel::Sender;
use rand::Rng;
use terminus_types::{Author, Node};

pub(super) fn post_panel(info: Option<&str>) -> Panel {
    let inputs = vec![
        Input::new("title", "言って", false),
        // もっと、もっと、もっと、ちゃんと言って
        Input::new("content", "", true),
    ];
    Panel::new(
        inputs,
        info.unwrap_or("press i to input, ESC to quit, Return to confirm."),
        PanelMode::Panel,
    )
}

pub(super) fn update_panel(info: Option<&str>, node: &Node) -> Panel {
    let inputs = vec![
        Input::new("title", &node.title, false),
        // もっと、もっと、もっと、ちゃんと言って
        Input::new("content", &node.content, true),
    ];
    Panel::new(
        inputs,
        info.unwrap_or("press i to input, ESC to quit, s to confirm."),
        PanelMode::Panel,
    )
}

/// node_id: parent id
pub(super) fn post_node(s: &Sender<Request>, id: &[u8], inputs: &[Input], author: Author) -> anyhow::Result<()> {
    let mut rand = rand::thread_rng();
    let tail: u64 = rand.gen();
    let mut node = Node::new(id, "title".to_string(), author, "content".to_string(), tail);
    for Input { label, input, .. } in inputs {
        match label.as_str() {
            "title" => node.title = input.to_string(),
            "content" => node.content = input.to_string(),
            _ => unreachable!(),
        }
    }
    let req = Request::Post(node);
    req.send(s)?;
    Ok(())
}

pub(super) fn update_node(s: &Sender<Request>, mut node: Node, inputs: &[Input], author: Author) -> anyhow::Result<()> {
    for Input { label, input, .. } in inputs {
        match label.as_str() {
            "title" => node.title = input.to_string(),
            "content" => node.content = input.to_string(),
            _ => unreachable!(),
        }
    }
    node.author = author;
    let req = Request::Update(node);
    req.send(s)?;
    Ok(())
}

pub(super) fn delete_confirm(node_hint: &str) -> Panel {
    Panel::new(
        vec![],
        format!("Do you really want to delete {}? [Y/n]", node_hint),
        PanelMode::Info,
    )
}

pub(super) fn delete_node(s: &Sender<Request>, mut node: Node, author: Author) -> anyhow::Result<()> {
    node.author = author;
    let req = Request::Delete(node);
    req.send(s)?;
    Ok(())
}
