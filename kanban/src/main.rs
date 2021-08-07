use config::Config;
use crossbeam_channel::Sender;
use message::Update;
use std::{
    fs,
    fs::OpenOptions,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

mod config;
mod event;
mod message;
mod store;
mod ui;

fn set_resize_info(s: Sender<Update>) -> anyhow::Result<()> {
    let mut hook = signal_hook::iterator::Signals::new(&[libc::SIGWINCH])?;
    thread::spawn(move || {
        for _ in hook.forever() {
            s.send(Update::Resize).ok();
        }
    });
    Ok(())
}

fn main() {
    // log file
    let log_dir_path = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join(config::APPLICATION);
    fs::create_dir_all(&log_dir_path).ok();
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir_path.join("client.log"))
        .expect("log file open failed");
    log_panics::init();
    let log_level = simplelog::LevelFilter::Info;
    let log_config = simplelog::ConfigBuilder::new().set_time_format_str("%+").build();
    simplelog::WriteLogger::init(log_level, log_config, log_file).expect("log set failed");

    let config = Config::from_file().unwrap_or_default();
    let config = Arc::new(Mutex::new(config));
    let (s_main, r_back) = crossbeam_channel::unbounded();
    let (s_back, r_main) = crossbeam_channel::unbounded();
    let s_event = s_back.clone();
    let event_th = thread::spawn(move || {
        if let Err(e) = event::handle(s_event) {
            log::error!("backend event failed: {}", e);
        }
    });
    let s_resize = s_back.clone();
    set_resize_info(s_resize).ok();
    let msg_config = config.clone();
    let message_th = thread::spawn(move || {
        if let Err(e) = message::handle(s_back, r_back, msg_config) {
            log::error!("backend message failed: {}", e);
        }
    });
    if let Err(e) = ui::run(s_main, r_main, config.clone()) {
        log::error!("tui failed: {}", e);
    }
    config.lock().unwrap().save_to_file().ok();
    event_th.join().ok();
    message_th.join().ok();
}
