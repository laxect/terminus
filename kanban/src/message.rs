use crossbeam_channel::{Receiver, Sender};
use std::{thread::sleep, time::Duration};
use terminus_types::{
    action::{Action, ListTarget, Response},
    Error, Node, NodeId,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp::OwnedReadHalf, TcpStream},
    runtime::Runtime,
    time::timeout,
};

#[derive(Debug)]
pub(crate) enum Request {
    Relink,
    ListRoot,
    Post(Node),
    Update(Node),
    Delete(Node),
    List(NodeId),
    // graceful exit,
    Shutdown,
}

impl Request {
    pub(crate) fn send(self, s: &Sender<Request>) -> anyhow::Result<()> {
        s.send(self)?;
        Ok(())
    }
}

impl From<Request> for Action {
    fn from(req: Request) -> Self {
        match req {
            Request::ListRoot => Self::List(ListTarget::Root),
            Request::List(id) => Self::List(ListTarget::Node(id)),
            Request::Post(node) => Self::Post(node),
            Request::Update(node) => Self::Update(node),
            Request::Delete(node) => Self::Delete(node),
            _ => unreachable!(),
        }
    }
}

impl Request {
    /// Returns `true` if the request is [`Shutdown`].
    pub(crate) fn is_shutdown(&self) -> bool {
        matches!(self, Self::Shutdown)
    }
}

#[derive(Debug)]
pub(crate) enum Update {
    Quit,
    Next,
    Prev,
    Parent,
    Child,
    Nodes(Vec<Node>),
    DeleteNode(Node),
    Err(Error),
}

/// link start!
/// may upgrade to tls in future.
async fn link_start() -> anyhow::Result<TcpStream> {
    Ok(TcpStream::connect("[::1]:1120").await?)
}

/// receive from remote.
/// can deal with subscription.
async fn receive(s: Sender<Update>, mut read: OwnedReadHalf) -> anyhow::Result<()> {
    let mut indicator = [0u8; 4];
    let mut buf = Vec::new();
    loop {
        read.read_exact(&mut indicator).await?;
        let size: u32 = bincode::deserialize(&indicator)?;
        if size == 0 {
            log::info!("end signal received.");
            return Ok(());
        }
        buf.resize(size as usize, 0u8);
        read.read_exact(&mut buf).await?;
        let update: Response = bincode::deserialize(&buf)?;
        match update {
            Response::Err(e) => {
                s.send(Update::Err(e))
                    .expect("sender droped which should not drop here.");
            }
            Response::List(list) => {
                s.send(Update::Nodes(list))
                    .expect("sender droped which should not drop here.");
            }
            Response::Delete(node) => {
                log::info!("operation delete success.");
                s.send(Update::DeleteNode(node))
                    .expect("sender droped which should not drop here.");
            }
            Response::Post(node) | Response::Update(node) => {
                log::info!("operation post/update success.");
                s.send(Update::Nodes(vec![node]))
                    .expect("sender droped which should not drop here.");
                // do nothing
            }
        }
    }
}

// u32 0
const EOS: &[u8] = &[0; 4];

async fn send(s: Sender<Update>, r: Receiver<Request>) -> anyhow::Result<()> {
    let (read, mut write) = link_start().await?.into_split();
    let recv_task = tokio::spawn(receive(s, read));
    while let Ok(req) = r.recv() {
        if req.is_shutdown() {
            break;
        }
        let action: Action = req.into();
        let size: u32 = bincode::serialized_size(&action)? as u32;
        let size = bincode::serialize(&size)?;
        let bin = bincode::serialize(&action)?;
        write.write_all(&size).await?;
        write.write_all(&bin).await?;
    }
    write.write(EOS).await?;
    if let Err(e) = timeout(Duration::from_secs(3), recv_task).await {
        log::error!("recv task exist with error: {}", e);
    }
    Ok(())
}

pub(crate) fn handle(s: Sender<Update>, r: Receiver<Request>) -> anyhow::Result<()> {
    let async_rt = Runtime::new().expect("runtime start up failed");
    loop {
        if let Err(e) = async_rt.block_on(send(s.clone(), r.clone())) {
            log::error!("link failed: {}", e);
            let error = Update::Err(Error::NetworkError);
            s.send(error).unwrap();
        } else {
            return Ok(());
        }
        // until a relink request
        loop {
            let req = r.recv()?;
            if req.is_shutdown() {
                return Ok(());
            }
            if matches!(req, Request::Relink) {
                break;
            }
        }
    }
}
