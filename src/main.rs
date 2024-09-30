pub mod file;
pub mod network;
pub mod swarm;
pub mod peer;
pub mod block;
use block::{
  Chain, Block
};

// RUST_LOG=info cargo run --bin main
#[tokio::main]
async fn main() {
  pretty_env_logger::init();

  run_p2p().await;
  // run_blocks()
}

async fn run_p2p(){
  let mut peer = peer::set_up_peer().await;
  peer.run().await
}

fn run_blocks(){
  let mut chain: Chain = Chain::new();
  for _ in 0 .. 10 {
    chain.make_new_valid_block("test".to_string());
  }
  println!("{}", chain);
}