pub mod file;
pub mod network;
pub mod swarm;
pub mod peer;

#[tokio::main]
async fn main() {
  pretty_env_logger::init();

  let mut peer = peer::set_up_peer().await;
  peer.run().await
}
