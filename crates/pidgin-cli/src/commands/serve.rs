use std::net::SocketAddr;
use std::path::PathBuf;

pub async fn run(bind: SocketAddr, host: PathBuf) {
    if let Err(e) = pidgin_server::serve(bind, host).await {
        eprintln!("server error: {}", e);
        std::process::exit(1);
    }
}
