use anyhow::Result;
use sled::{Config, Db};
use terminus_types::{Node, NodeId};

pub(crate) struct Store {
    inner: Db,
}

const ROOT_START: &[u8] = &[0; 16];
const ROOT_END: &[u8] = &[255; 16];
impl Store {
    pub(crate) fn new() -> Result<Self> {
        let config = Config::new()
            .temporary(true)
            .use_compression(false)
            .mode(sled::Mode::HighThroughput);
        Ok(Self { inner: config.open()? })
    }

    /// can post/update
    pub(crate) fn insert(&self, node: Node) -> Result<()> {
        self.inner.insert(&node.id, bincode::serialize(&node)?)?;
        Ok(())
    }

    pub(crate) fn delete(&self, node: Node) -> Result<()> {
        self.inner.remove(node.id)?;
        Ok(())
    }

    pub(crate) fn list_root(&self) -> Result<Vec<Node>> {
        let mut res = Vec::new();
        let list = self.inner.range(ROOT_START..ROOT_END).values();
        for item in list {
            let item = bincode::deserialize(&item?)?;
            res.push(item);
        }
        res.sort_unstable_by_key(|node: &Node| node.last_reply);
        Ok(res)
    }

    pub(crate) fn list(&self, root: &NodeId) -> Result<Vec<Node>> {
        let mut res = Vec::new();
        let list = self.inner.scan_prefix(root).values();
        for item in list {
            let item = bincode::deserialize(&item?)?;
            res.push(item);
        }
        res.sort_by_cached_key(|node: &Node| {
            let mut id: Vec<u128> = Vec::new();
            let mut n = 0;
            while let Some(slice) = node.id.get(n..n + 16) {
                if let Ok(layer) = bincode::deserialize(slice) {
                    id.push(layer);
                } else {
                    log::error!("data deserialize failed!");
                }
                n += 4;
            }
            id
        });
        Ok(res)
    }
}
