use std::{fs::OpenOptions, thread};

use message::Request;

mod event;
mod message;
mod store;
mod ui;

fn main() {
    // log file
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("./kanban.log")
        .expect("log file open failed");
    log_panics::init();
    let log_level = simplelog::LevelFilter::Info;
    let log_config = simplelog::ConfigBuilder::new().set_time_format_str("%+").build();
    simplelog::WriteLogger::init(log_level, log_config, log_file).expect("log set failed");

    let (s_main, r_back) = crossbeam_channel::unbounded();
    let (s_back, r_main) = crossbeam_channel::unbounded();
    let s_event = s_back.clone();
    let event_th = thread::spawn(move || {
        if let Err(e) = event::handle(s_event) {
            log::error!("backend event failed: {}", e);
        }
    });
    let message_th = thread::spawn(move || {
        if let Err(e) = message::handle(s_back, r_back) {
            log::error!("backend message failed: {}", e);
        }
    });
    let req = Request::ListRoot;
    req.send(&s_main).expect("inital list failed.");
    if let Err(e) = ui::run(s_main, r_main) {
        log::error!("tui failed: {}", e);
    }
    event_th.join().ok();
    message_th.join().ok();
}
