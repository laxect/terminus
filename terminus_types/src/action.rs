use crate::{Error, Node, NodeId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ListTarget {
    Root,
    Node(NodeId),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Action {
    Delete(Node),
    List(ListTarget),
    Update(Node),
    Post(Node),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Post(Node),
    Update(Node),
    Delete(Node),
    List(Vec<Node>),
    Err(Error),
}
