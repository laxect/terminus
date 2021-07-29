use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
};

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
        let ss: String = bincode::deserialize(&buf)?;
        println!("{}", ss);
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
        log::info!("Link in from: {}", address);
        tokio::spawn(handle(socket));
    }
}
