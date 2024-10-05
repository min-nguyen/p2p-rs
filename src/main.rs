pub mod file;
pub mod peer;
pub mod block;
pub mod message;
pub mod swarm_flood;
pub mod swarm_gossip;

use block::{
  Chain, Block
};

// RUST_LOG=info cargo run --bin main
#[tokio::main]
async fn main() {
  pretty_env_logger::init();
  let (x, y) = (hex::encode([0;32]), hex::encode([0]));
  println!("{}, {}, {:?}, {:?}", x, y, hex::decode(&x), hex::decode(&y));

  run_p2p().await;
  // dummy_chain(10)
}

async fn run_p2p(){
  let mut peer = peer::set_up_peer().await;
  peer.run().await
}

fn dummy_chain(len : u32) -> Chain{
  let mut chain: Chain = Chain::new();
  for _ in 0 .. len {
    chain.make_new_valid_block("test");
  }
  println!("{}", chain);
  chain
}
