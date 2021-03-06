use chrono::{DateTime, Duration, Utc};
use crossbeam_channel::Sender;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sled::{Batch, Db, IVec};
use terminus_types::{action::Response, Author, Error, Node, NodeId};

#[derive(Deserialize, Serialize, Clone)]
struct NodeBody {
    pub title: String,
    pub author: Author,
    pub content: String,
    pub publish_time: DateTime<Utc>,
    pub last_reply: DateTime<Utc>,
    pub edited: bool,
}

impl NodeBody {
    pub(crate) fn match_pass(&self, other: &NodeBody) -> bool {
        let name = &other.author.name;
        let pass = other.author.pass.get_pass();
        self.author.match_pass(name, pass)
    }

    pub(crate) fn update_publish_time(&mut self) {
        self.publish_time = Utc::now();
    }

    pub(crate) fn mask(&mut self) {
        self.author.mask()
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
        last_reply,
        edited,
    } = node;
    let body = NodeBody {
        title,
        author,
        content,
        publish_time,
        last_reply,
        edited,
    };
    Ok((id, body))
}

fn assemble_node(id: &[u8], body: &[u8]) -> anyhow::Result<Node> {
    let NodeBody {
        title,
        author,
        content,
        publish_time,
        last_reply,
        edited,
    } = bincode::deserialize(body)?;
    Ok(Node {
        id: id.to_owned(),
        title,
        author,
        content,
        publish_time,
        last_reply,
        edited,
    })
}

const CONTENT_TREE: &str = "content";
const ROOT_LIST: &str = "root_list";
static DB: Lazy<Db> = Lazy::new(|| sled::open("database").unwrap());

pub(crate) fn post(mut node: Node) -> anyhow::Result<Response> {
    node.author.mask();
    let resp_node = node.clone();
    if node.id.len() % 16 != 0 {
        return Ok(Response::Err(Error::IdInvalid));
    }
    // TODO may should update last id
    // should have one
    let node_id = node.last_id()?;
    log::info!("[post] new post: {}.", node_id);
    // should have one
    let top_id_bin = node.top_id_bin()?;
    let is_top_level = node.is_top_level();
    // disperse node
    let (id, mut body) = disperse_node(node)?;
    body.update_publish_time();
    let tree = DB.open_tree(CONTENT_TREE)?;
    let mut batch = Batch::default();
    if tree.contains_key(&id)? {
        return Ok(Response::Err(Error::NodeExist));
    }
    // check top level only when this is not a top level node
    if !is_top_level {
        let top = tree.get(&top_id_bin)?;
        let mut top: NodeBody = if let Some(top) = top {
            bincode::deserialize(&top)?
        } else {
            return Ok(Response::Err(Error::IdInvalid));
        };
        top.last_reply = body.publish_time;
        DB.open_tree(ROOT_LIST)?.insert(&top_id_bin, top.clone())?;
        batch.insert(top_id_bin, top);
    } else {
        let root_list = DB.open_tree(ROOT_LIST)?;
        root_list.insert(id.clone(), body.clone())?;
    }
    batch.insert(id, body);
    tree.apply_batch(batch)?;
    Ok(Response::Post(resp_node))
}

pub(crate) fn list_root() -> anyhow::Result<Response> {
    let tree = DB.open_tree(ROOT_LIST)?;
    let list = tree.iter();
    let mut res = Vec::new();
    for item in list {
        let (id, body) = item?;
        res.push(assemble_node(&id, &body)?);
    }
    Ok(Response::List(res))
}

pub(crate) fn list(root: NodeId) -> anyhow::Result<Response> {
    let tree = DB.open_tree(CONTENT_TREE)?;
    let list = tree.scan_prefix(root);
    let mut res = Vec::new();
    for item in list {
        let (id, body) = item?;
        res.push(assemble_node(&id, &body)?);
    }
    Ok(Response::List(res))
}

fn delete_or_update<F>(node: Node, action: &str, action_fun: F) -> anyhow::Result<Response>
where
    F: Fn(&sled::Tree, &[u8], NodeBody, NodeBody) -> anyhow::Result<Response>,
{
    if node.author.is_masked() {
        return Ok(Response::Err(Error::NeedUnMaskPass));
    }
    let target_id = node.last_id()?;
    log::info!("[{}] node {}.", action, target_id);
    // prepare
    let (id, mut body) = disperse_node(node)?;
    // really do
    let tree = DB.open_tree(CONTENT_TREE)?;
    if let Some(old_body) = tree.get(&id)? {
        let old_body: NodeBody = bincode::deserialize(&old_body)?;
        if old_body.match_pass(&body) {
            body.mask();
            let resp = action_fun(&tree, &id, body, old_body)?;
            return Ok(resp);
        }
        log::warn!("[{}] node {} pass not match.", action, target_id);
        return Ok(Response::Err(Error::PassNotMatch));
    }
    log::warn!("[{}] node {} not found.", action, target_id);
    Ok(Response::Err(Error::NodeNotExist))
}

pub(crate) fn delete(node: Node) -> anyhow::Result<Response> {
    let five_hour = Duration::hours(5);
    delete_or_update(node, "delete", |tree, id, body, old_body| {
        let publish_time = old_body.publish_time;
        let now = Utc::now();
        if now - publish_time > five_hour {
            return Ok(Response::Err(Error::DeleteLimitOverdue));
        }
        let node = assemble_node(id, &bincode::serialize(&body)?)?;
        tree.remove(id)?;
        if node.is_top_level() {
            DB.open_tree(ROOT_LIST)?.remove(id)?;
        }
        Ok(Response::Delete(node))
    })
}

pub(crate) fn update(node: Node) -> anyhow::Result<Response> {
    delete_or_update(node, "update", |tree, id, mut body, _old_body| {
        body.edited = true;
        let node = assemble_node(id, &bincode::serialize(&body)?)?;
        // TODO may should use merge instead of replace
        tree.insert(id, body.clone())?;
        if node.is_top_level() {
            DB.open_tree(ROOT_LIST)?.insert(id, body)?;
        }
        Ok(Response::Update(node))
    })
}

pub(crate) async fn notify_channel(s: Sender<Response>) -> anyhow::Result<()> {
    let mut inbox = DB.open_tree(CONTENT_TREE)?.watch_prefix(&[]);
    while let Some(event) = (&mut inbox).await {
        match event {
            sled::Event::Insert { key, value } => {
                let node = assemble_node(&key, &value)?;
                let resp = Response::Update(node);
                s.send(resp)?;
            }
            sled::Event::Remove { key } => {
                let mut node = Node::empty_node();
                node.id = key.to_vec();
                let resp = Response::Delete(node);
                s.send(resp)?;
            }
        }
    }
    Ok(())
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
