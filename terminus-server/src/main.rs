use terminus_types::action::{Action, ListTarget, Response};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
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

async fn handle(mut socket: TcpStream) -> anyhow::Result<()> {
    let mut indicator = [0u8; 4];
    let mut buf = Vec::new();

    loop {
        socket.read_exact(&mut indicator).await?;
        let size: u32 = bincode::deserialize(&indicator)?;
        if size == 0 {
            return Ok(());
        }
        buf.resize(size as usize, 0u8);
        socket.read_exact(&mut buf).await?;
        let action: Action = bincode::deserialize(&buf)?;
        match take_action(action) {
            Ok(resp) => {
                let size = bincode::serialized_size(&resp)? as u32;
                let size = bincode::serialize(&size)?;
                let data = bincode::serialize(&resp)?;
                socket.write_all(&size).await?;
                socket.write_all(&data).await?;
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

    let listener = TcpListener::bind("[::]:3000").await?;

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
