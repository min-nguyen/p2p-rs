use libp2p::Swarm;
use log::info;
use super::local_network::{RECIPE_TOPIC, RecipeBehaviour, RecipeResponse, RecipeRequest, TransmitType};


pub async fn publish_response(resp: RecipeResponse, swarm: &mut Swarm<RecipeBehaviour>){
  let json = serde_json::to_string(&resp).expect("can jsonify response");
  publish(json, swarm).await
}
pub async fn publish_request(resp: RecipeRequest, swarm: &mut Swarm<RecipeBehaviour>){
  let json = serde_json::to_string(&resp).expect("can jsonify response");
  publish(json, swarm).await
}
pub async fn handle_req_recipes(cmd: &str, swarm: &mut Swarm<RecipeBehaviour>) {
  let args: Option<&str>
      = cmd.strip_prefix("req")
           .map(|rest: &str| rest.trim());
  match args {
      Some("") | None => {
          info!("Command error: [req] missing an argument, specify \"all\" or \"<peer_id>\"")
      }
      // `req r all` send a request for all recipes from all known peers
      Some("all") => {
          let req = RecipeRequest {
              transmit_type: TransmitType::ToAll,
          };
          publish_request(req, swarm).await
      }
      // `req r <peerId>` sends a request for all recipes from a certain peer
      Some(peer_id) => {
          let req = RecipeRequest {
              transmit_type: TransmitType::ToOne(peer_id.to_owned()),
          };
          publish_request(req, swarm).await
      }
  };
}
async fn publish(json : String,  swarm: &mut Swarm<RecipeBehaviour> ) {
  swarm
      .behaviour_mut()
      .floodsub
      .publish(RECIPE_TOPIC.clone(), json.as_bytes());
}