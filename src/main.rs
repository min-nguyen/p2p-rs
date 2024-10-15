pub mod cryptutil;
pub mod file;
pub mod peer;
pub mod block;
pub mod chain;
pub mod transaction;
pub mod message;
pub mod swarm;

use chain::Chain;

// RUST_LOG=info cargo run --bin main
#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    run_p2p().await;
    // dummy_chain(10)
}

async fn run_p2p(){
    let mut peer = peer::set_up_peer().await;
    peer.run().await
}

fn dummy_chain(len : u32) -> Chain{
    let mut chain: Chain = Chain::genesis();
    for _ in 0 .. len {
      chain.mine_then_push_block("test");
    }
    println!("{}", chain);
    chain
}
