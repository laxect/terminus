use anyhow::Result;
use sled::{Config, Db};
use terminus_types::Node;

pub(crate) struct Store {
    inner: Db,
}

const ROOT_LIST: &str = "root_list";
impl Store {
    pub(crate) fn new() -> Result<Self> {
        let config = Config::new()
            .temporary(true)
            .use_compression(false)
            .mode(sled::Mode::HighThroughput);
        Ok(Self { inner: config.open()? })
    }

    pub(crate) fn clear(&self) {
        self.inner.clear().ok();
        self.inner.open_tree(ROOT_LIST).unwrap().clear().ok();
    }

    /// can post/update
    pub(crate) fn insert(&self, node: Node) -> Result<()> {
        let value = bincode::serialize(&node)?;
        self.inner.insert(&node.id, value.clone())?;
        if node.is_top_level() {
            self.inner.open_tree(ROOT_LIST)?.insert(&node.id, value)?;
        }
        Ok(())
    }

    pub(crate) fn delete(&self, node: &Node) -> Result<()> {
        if node.is_top_level() {
            self.inner.open_tree(ROOT_LIST).unwrap().remove(&node.id)?;
        }
        self.inner.remove(&node.id)?;
        Ok(())
    }

    pub(crate) fn list_root(&self) -> Result<Vec<Node>> {
        let mut res = Vec::new();
        let list = self.inner.open_tree(ROOT_LIST)?.iter();
        for item in list {
            let (_key, val) = item?;
            let item = bincode::deserialize(&val)?;
            res.push(item);
        }
        res.sort_unstable_by_key(|node: &Node| node.last_reply);
        res.reverse();
        Ok(res)
    }

    pub(crate) fn list(&self, root: &[u8]) -> Result<Vec<Node>> {
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
