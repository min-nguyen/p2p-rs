
pub mod peer;
pub mod file;
pub mod network;
pub mod swarm;

#[tokio::main]
async fn main() {
  pretty_env_logger::init();

  let mut peer = peer::set_up_peer().await;
  peer.handle_local_events().await
}
