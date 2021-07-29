use crate::{Node, NodeId};
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

/// TODO
#[derive(Serialize, Deserialize, Debug)]
pub enum Response {}
