use super::panel::{Panel, PanelMode};
use crate::{
    message::{self, Request},
    ui::panel::Input,
};
use crossbeam_channel::Sender;
use rand::Rng;
use terminus_types::{Author, Node};

pub(super) fn edit_panel(info: Option<&str>) -> Panel {
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

/// node_id: parent id
pub(super) fn post_node_update(
    s: &Sender<message::Request>,
    node_id: &[u8],
    inputs: &[Input],
    author: Author,
) -> anyhow::Result<()> {
    let mut rand = rand::thread_rng();
    let tail: u64 = rand.gen();
    let mut node = Node::new(node_id, "title".to_string(), author, "content".to_string(), tail);
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
