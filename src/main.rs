mod p2p {
  pub mod p2p;
  pub mod recipes;
  pub mod behaviour;
}

#[tokio::main]
async fn main() {
  p2p::p2p::set_up_peer().await
}
