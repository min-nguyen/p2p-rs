use powp2p;

// RUST_LOG=info cargo run --bin main
#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let mut peer = powp2p::peer::set_up_peer().await;
    peer.run().await
}
