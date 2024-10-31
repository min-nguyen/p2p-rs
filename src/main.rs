#[macro_use]
pub mod util;
pub mod block;
pub mod chain;
pub mod crypt;
pub mod file;
pub mod fork;
pub mod message;
pub mod peer;
pub mod swarm;
pub mod transaction;
pub mod tests {
    pub mod block;
    pub mod chain;
    pub mod transaction;
}

// RUST_LOG=info cargo run --bin main
#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let mut peer = peer::set_up_peer().await;
    peer.run().await
}
