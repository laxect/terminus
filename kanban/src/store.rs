use anyhow::Result;
use sled::{Config, Db};
use terminus_types::{Node, NodeId};

pub(crate) struct Store {
    inner: Db,
}

const ROOT_START: &[u8] = &[0; 24];
const ROOT_END: &[u8] = &[255; 24];
impl Store {
    pub(crate) fn new() -> Result<Self> {
        let config = Config::new()
            .temporary(true)
            .use_compression(false)
            .mode(sled::Mode::HighThroughput);
        Ok(Self { inner: config.open()? })
    }

    pub(crate) fn insert(&self, node: Node) -> Result<()> {
        let id = bincode::serialize(&node.id)?;
        let node = bincode::serialize(&node)?;
        self.inner.insert(id, node)?;
        Ok(())
    }

    pub(crate) fn list_root(&self) -> Result<Vec<Node>> {
        let mut res = Vec::new();
        let list = self.inner.range(ROOT_START..ROOT_END).values();
        for item in list {
            let item = bincode::deserialize(&item?)?;
            res.push(item);
        }
        Ok(res)
    }

    pub(crate) fn list(&self, root: NodeId) -> Result<Vec<Node>> {
        let mut res = Vec::new();
        let list = self.inner.range(ROOT_START..ROOT_END).values();
        for item in list {
            let item = bincode::deserialize(&item?)?;
            res.push(item);
        }
        Ok(res)
    }
}
