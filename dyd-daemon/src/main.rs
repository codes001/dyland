use dyd_daemon::Daemon;

#[tokio::main]
async fn main() {
    let mut daemon = Daemon::new();
    daemon.run().await;
}
