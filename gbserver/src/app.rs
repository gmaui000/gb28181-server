use std::net::UdpSocket;

use common::daemon::Daemon;
use common::dbx::mysqlx;
use common::exception::{GlobalError, GlobalResult, TransError};
use common::log::{error, info};
use common::logger;
use common::tokio;

use crate::gb::SessionConf;
use crate::general::http::Http;

#[derive(Debug)]
pub struct AppInfo {
    session_conf: SessionConf,
    http: Http,
}

impl
    Daemon<(
        std::net::TcpListener,
        (Option<std::net::TcpListener>, Option<UdpSocket>),
    )> for AppInfo
{
    fn init_privilege() -> GlobalResult<(
        Self,
        (
            std::net::TcpListener,
            (Option<std::net::TcpListener>, Option<UdpSocket>),
        ),
    )>
    where
        Self: Sized,
    {
        let app_info = AppInfo {
            session_conf: SessionConf::get_session_by_conf(),
            http: Http::get_http_by_conf(),
        };
        logger::Logger::init()?;
        banner();
        let http_listener = app_info.http.listen_http_server()?;
        let tu = app_info.session_conf.listen_gb_server()?;
        Ok((app_info, (http_listener, tu)))
    }

    fn run_app(
        self,
        t: (
            std::net::TcpListener,
            (Option<std::net::TcpListener>, Option<UdpSocket>),
        ),
    ) -> GlobalResult<()> {
        let http = self.http;
        let (http_listener, tu) = t;
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                mysqlx::init_conn_pool()?;
                let web = tokio::spawn(async move {
                    info!("Web server start running...");
                    http.run(http_listener).await?;
                    error!("Web server stop");
                    Ok::<(), GlobalError>(())
                });
                let se = tokio::spawn(async move {
                    info!("Session server start running...");
                    SessionConf::run(tu).await?;
                    error!("Session server stop");
                    Ok::<(), GlobalError>(())
                });
                se.await.hand_log(|msg| error!("Session:{msg}"))??;
                web.await.hand_log(|msg| error!("WEB:{msg}"))??;
                Ok::<(), GlobalError>(())
            })?;
        error!("系统异常退出...");
        Ok(())
    }
}

fn banner() {
    let br = r#"
              ____  ____  ____   ___   _   ___   _        ____   _____  ____ __     __ _____  ____  
             / ___|| __ )|___ \ ( _ ) / | ( _ ) / |      / ___| | ____||  _ \\ \   / /| ____||  _ \ 
    o O O   | |  _ |  _ \  __) |/ _ \ | | / _ \ | | _____\___ \ |  _|  | |_) |\ \ / / |  _|  | |_) |
   o        | |_| || |_) |/ __/| (_) || || (_) || ||_____|___) || |___ |  _ <  \ V /  | |___ |  _ < 
  o0__[O]    \____||____/|_____|\___/ |_| \___/ |_|      |____/ |_____||_| \_\  \_/   |_____||_| \_\
"#;
    info!("{}", br);
}
