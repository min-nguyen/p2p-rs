
pub mod util;
pub mod file;
pub mod peer;
pub mod chain;
pub mod transaction;
pub mod message;
pub mod swarm_flood;
pub mod swarm_gossip;

use chain::Chain;

// RUST_LOG=info cargo run --bin main
#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    // let (x, y) = (util::encode_hex(util::ZERO_U64), util::encode_hex([0; 64]));
    // println!("{}, {}, {:?}, {:?}", x, y, util::decode_hex(&x, 64).unwrap().len(), util::decode_hex(&y, 32));

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
