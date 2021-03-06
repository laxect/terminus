use crossbeam_channel::Receiver;
use terminus_types::action::{Action, ListTarget, Response};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp::OwnedWriteHalf, TcpListener, TcpStream},
};

mod store;

fn take_action(action: Action) -> anyhow::Result<Response> {
    match action {
        Action::Post(node) => store::post(node),
        Action::Delete(node) => store::delete(node),
        Action::Update(node) => store::update(node),
        Action::List(ListTarget::Root) => store::list_root(),
        Action::List(ListTarget::Node(node_id)) => store::list(node_id),
    }
}

async fn send(mut send: OwnedWriteHalf, recv: Receiver<Response>) -> anyhow::Result<()> {
    loop {
        let resp = recv.recv()?;
        let size = bincode::serialized_size(&resp)? as u32;
        let size = bincode::serialize(&size)?;
        let data = bincode::serialize(&resp)?;
        send.write_all(&size).await?;
        send.write_all(&data).await?;
    }
}

async fn handle(link: TcpStream) -> anyhow::Result<()> {
    let mut indicator = [0u8; 4];
    let mut buf = Vec::new();
    let (mut r, s) = link.into_split();
    let (client_s, client_r) = crossbeam_channel::unbounded();
    let to_client = tokio::spawn(send(s, client_r));
    let inbox = tokio::spawn(store::notify_channel(client_s.clone()));
    loop {
        r.read_exact(&mut indicator).await?;
        let size: u32 = bincode::deserialize(&indicator)?;
        if size == 0 {
            log::info!("end signal received.");
            to_client.abort();
            inbox.abort();
            return Ok(());
        }
        buf.resize(size as usize, 0u8);
        r.read_exact(&mut buf).await?;
        let action: Action = bincode::deserialize(&buf)?;
        match take_action(action) {
            Ok(resp) => {
                if let Response::Err(e) = &resp {
                    log::warn!("[handle] node handle failed: {}.", e);
                }
                client_s.send(resp)?;
            }
            Err(e) => {
                log::warn!("can not deal request: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // logs
    log_panics::init();
    let log_level = simplelog::LevelFilter::Info;
    let log_config = simplelog::ConfigBuilder::new().set_time_format_str("%+").build();
    let term_mode = simplelog::TerminalMode::Stdout;
    let color_choice = simplelog::ColorChoice::Auto;
    simplelog::TermLogger::init(log_level, log_config, term_mode, color_choice).expect("log set failed");

    let listener = TcpListener::bind("[::]:1120").await?;

    loop {
        let (socket, address) = listener.accept().await?;
        log::info!("Link from {} established.", address);
        tokio::spawn(async move {
            if let Err(e) = handle(socket).await {
                log::warn!("link from {} failed: {}.", address, e);
            } else {
                log::info!("link from {} ended.", address);
            }
        });
    }
}
