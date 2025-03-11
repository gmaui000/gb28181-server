use crate::app::AppInfo;
use common::daemon::Daemon;

// #![allow(warnings)]
mod app;
pub mod gb;
pub mod general;
mod service;
pub mod storage;
mod utils;
mod web;

fn main() {
    let config_path = "config.yml";
    common::confgen::conf::init_confgen(config_path.to_string());

    // daemon::run::<AppInfo, _>();
    if let Ok((appinfo, (http_listener, (tcp_listener, udp_socket)))) = AppInfo::init_privilege() {
        let _ = appinfo.run_app((http_listener, (tcp_listener, udp_socket)));
    }
}
