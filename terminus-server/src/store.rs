use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sled::{Db, IVec};
use std::sync::atomic::{AtomicU64, Ordering};
use terminus_types::{action::Response, Author, Error, Node, NodeId as OriNodeId};

type NodeId = Vec<u8>;

#[derive(Deserialize, Serialize)]
struct NodeBody {
    pub title: String,
    pub author: Author,
    pub content: String,
    pub publish_time: DateTime<Utc>,
    pub edited: bool,
}

impl NodeBody {
    pub(crate) fn match_pass(&self, other: &NodeBody) -> bool {
        let name = &other.author.name;
        let pass = other.author.pass.get_pass();
        self.author.match_pass(name, pass)
    }
}

impl From<NodeBody> for IVec {
    fn from(val: NodeBody) -> Self {
        bincode::serialize(&val)
            .expect("should always serialize success")
            .into()
    }
}

fn disperse_node(node: Node) -> anyhow::Result<(NodeId, NodeBody)> {
    let Node {
        id,
        title,
        author,
        content,
        publish_time,
        edited,
    } = node;
    let id = bincode::serialize(&id)?;
    let body = NodeBody {
        title,
        author,
        content,
        publish_time,
        edited,
    };
    Ok((id, body))
}

fn assemble_node(id: &[u8], body: &[u8]) -> anyhow::Result<Node> {
    let id = bincode::deserialize(id)?;
    let NodeBody {
        title,
        author,
        content,
        publish_time,
        edited,
    } = bincode::deserialize(body)?;
    Ok(Node {
        id,
        title,
        author,
        content,
        publish_time,
        edited,
    })
}

const CONTENT_TREE: &str = "content";
static DB: Lazy<Db> = Lazy::new(|| sled::open("database").unwrap());
static COUNT: AtomicU64 = AtomicU64::new(0);

pub(crate) fn post(node: Node) -> anyhow::Result<Response> {
    // should have one
    let target_id: u128;
    if let Some(node_id) = node.id.last() {
        target_id = *node_id;
        log::info!("[post] new post: {}.", node_id);
    } else {
        return Ok(Response::Err(Error::IdInvalid));
    }
    let tree = DB.open_tree(CONTENT_TREE)?;
    let (id, body) = disperse_node(node)?;
    if tree.contains_key(&id)? {
        log::warn!("[post] attempt to duplicate node {}.", target_id);
        return Ok(Response::Err(Error::NodeExist));
    }
    tree.insert(&id, body)?;
    COUNT.fetch_add(1, Ordering::Relaxed);
    Ok(Response::Post)
}

const ROOT_START: &[u8] = &[0; 24];
const ROOT_END: &[u8] = &[255; 24];

pub(crate) fn list_root() -> anyhow::Result<Response> {
    let tree = DB.open_tree(CONTENT_TREE)?;
    let list = tree.range(ROOT_START..ROOT_END);
    let mut res = Vec::new();
    for item in list {
        let (id, body) = item?;
        res.push(assemble_node(&id, &body)?);
    }
    Ok(Response::List(res))
}

pub(crate) fn list(root: OriNodeId) -> anyhow::Result<Response> {
    let root_id = bincode::serialize(&root)?;
    let tree = DB.open_tree(CONTENT_TREE)?;
    let list = tree.scan_prefix(root_id);
    let mut res = Vec::new();
    for item in list {
        let (id, body) = item?;
        res.push(assemble_node(&id, &body)?);
    }
    Ok(Response::List(res))
}

fn delete_or_update<F>(node: Node, action: &str, action_fun: F) -> anyhow::Result<Response>
where
    F: Fn(&sled::Tree, &[u8], NodeBody) -> anyhow::Result<()>,
{
    if node.author.is_masked() {
        return Ok(Response::Err(Error::NeedUnMaskPass));
    }
    let target_id = node.last_id()?;
    log::info!("[{}] node {}.", action, target_id);
    // prepare
    let (id, body) = disperse_node(node)?;
    // really delete
    let tree = DB.open_tree(CONTENT_TREE)?;
    if let Some(old_body) = tree.get(&id)? {
        let old_body: NodeBody = bincode::deserialize(&old_body)?;
        if old_body.match_pass(&body) {
            action_fun(&tree, &id, body)?;
            return Ok(Response::Delete);
        }
        log::warn!("[{}] node {} pass not match.", action, target_id);
        return Ok(Response::Err(Error::PassNotMatch));
    }
    log::warn!("[{}] node {} not found.", action, target_id);
    Ok(Response::Err(Error::NodeNotExist))
}

pub(crate) fn delete(node: Node) -> anyhow::Result<Response> {
    delete_or_update(node, "delete", |tree, id, _body| {
        tree.remove(id)?;
        Ok(())
    })
}

pub(crate) fn update(node: Node) -> anyhow::Result<Response> {
    delete_or_update(node, "update", |tree, id, mut body| {
        body.edited = true;
        tree.insert(id, body)?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use crate::store::{assemble_node, disperse_node};

    use super::{Author, Node};

    #[test]
    fn disperse_and_assemble() {
        let me = Author::new("Me!".to_string(), "ME!ME!ME!".to_string());
        let node = Node::new(&[], "Hi".to_string(), me, "nothing".to_string(), 90);
        let origin = format!("{:?}", &node);
        let (id, body) = disperse_node(node).unwrap();
        let node = assemble_node(&id, &bincode::serialize(&body).unwrap()).unwrap();
        let back = format!("{:?}", &node);
        assert_eq!(origin, back);
    }
}
