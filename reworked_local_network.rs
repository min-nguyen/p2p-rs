use libp2p::{
  core::upgrade,
  floodsub::{Floodsub, FloodsubEvent, Topic},
  futures::StreamExt,
  identity,
  mdns::{Mdns, MdnsEvent},
  mplex,
  noise::{Keypair, NoiseConfig, X25519Spec},
  swarm::{NetworkBehaviourEventProcess, Swarm, SwarmBuilder},
  tcp::TokioTcpConfig,
  NetworkBehaviour, PeerId, Transport,
};
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::{io::AsyncBufReadExt, sync::mpsc};
use super::local_data;

// # *NetworkBehavior* (key concept in p2p): Defines the logic of the p2p network and all its peers.
//
// We need to specify at least 2 Protocol Types:
//   1. Communication Protocol between peers
//   2. Discovery Protocol for peers to find each other
// We need to define a Concrete Network Sub-Behaviour for each Protocol Type:
//   1. Handling network events regarding the Communication Protocol
//   2. Handling network events regarding the Discovery Protocol
//
// We will use the FloodSub Communication Protocol.
// This is a publish-subscribe protocol:
// - Publishers send messages to *all* peers they are directly connected to, without any filtering.
// - Subscribers receive messages by subscribing to specific topics.
// - When a message is published, it is flooded to all peers in the network, and
//   each peer forwards the message to their connected peers until the message reaches all interested nodes.
//
// We will use the mDNS Discovery Protocol.

// FloodSub Topic for subscribing and sending recipes
pub static RECIPE_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("recipes"));

// Messages can be intended for all peers or a specific peer.
#[derive(Debug, Serialize, Deserialize)]
enum TransmitType {
    ToAll,
    ToOne(String)   // contains intended peer id
}
// Messages are either (1) requests for data, or (2) responses to a request.
#[derive(Debug, Serialize, Deserialize)]
pub struct RecipeRequest {
    // Requests for recipes can be either ToAll or ToOne
    transmit_type : TransmitType
}
#[derive(Debug, Serialize, Deserialize)]
pub struct RecipeResponse {
    // Responding with our recipes will always to be ToAll
    transmit_type : TransmitType,
    // Core message payload being transmitted in the network.
    data : local_data::Recipes,
    // The PeerID to recieve the response.
    receiver : String
}

// Represents the base NetworkBehaviour, specifying the 2 Protocol types
#[derive(NetworkBehaviour)]
pub struct RecipeBehaviour {
    // ** Relevant to the global P2P Network Behaviour that all peers must share:
    // 1. A Communication Protocol Type between peers on the network.
    //    We will use the FloodSub protocol to deal with events in the network.
    pub floodsub: Floodsub,
    // 2. A Discovery Protocol Type for discovering new peers
    //    We will use the mDNS protocol for discovering other peers on the local network.
    mdns: Mdns,

    // ** Relevant only to a specific peer that we are setting up, and Irrelevant to the NetworkBehaviour:
    // 1. A channel to receive responses *from* the network, and forward these *to* the main part of our application elsewhere.
    //    We will use `response_sender` to send responses from the network to some paired `response_rcv` elsewhere in our program.
    #[behaviour(ignore)]
    local_sender: mpsc::UnboundedSender<RecipeResponse>,
    // 2. Our own PeerId
    #[behaviour(ignore)]
    local_peer_id: libp2p::PeerId
}
// Defining the Sub-Behaviours for handling events, `inject_event()`, from each Protocol Type.
// 1. Sub-Behaviour for the mDNS Discovery Protocol.
impl NetworkBehaviourEventProcess<MdnsEvent> for RecipeBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            // Event for discovering (a list of) new peers
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    // Add to our list of peers to propagate messages to
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            // Event for (a list of) expired peers
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    // Remove from our list of peers
                    info!("Removed Peer");
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
            //
            // MdnsEvent
        }
    }
}
// 2. Concrete Sub-Behaviour for the FloodSub Communication Protocol.
impl NetworkBehaviourEventProcess<FloodsubEvent> for RecipeBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        match event {
            // Event for a new message from a peer
            FloodsubEvent::Message(msg) => {
                // FloodSubMessage {
                //      msg.data   : Vec<u8, Global> -- message payload
                //      msg.source : PeerId          -- source peer id
                // }
                // 1. Pattern match on the payload as (successfully deserializing into) a RecipeResponse
                if let Ok(resp) = serde_json::from_slice::<RecipeResponse>(&msg.data) {
                    if resp.receiver == self.local_peer_id.to_string() {
                        info!("Response from {}:", msg.source);
                        resp.data.iter().for_each(|r| info!("{:?}", r));
                    }
                }
                // 2. Pattern match on the payload as (successfully deserializing into) a RecipeRequest
                else if let Ok(req) = serde_json::from_slice::<RecipeRequest>(&msg.data) {
                    match req.transmit_type {
                        // Handle a ToAll request intended for all peers
                        TransmitType::ToAll => {
                            info!("Received ToAll req {:?} from {:?}", req, msg.source);
                            respond_with_recipes(
                                self.local_sender.clone(),
                                msg.source.to_string(),
                            );
                        }
                        // Handle a ToOne request if it was intended for us
                        TransmitType::ToOne(ref peer_id) => {
                            info!("Received ToOne req {:?} from {:?}", req, msg.source);
                            if peer_id == &self.local_peer_id.to_string() {
                                info!("Handling ToOne req  from {:?}", msg.source);
                                respond_with_recipes(
                                    self.local_sender.clone(),
                                    msg.source.to_string(),
                                );
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }
}

// Helper for setting up a Recipe NetworkBehaviour that subscribes to the Recipe topic.
pub async fn set_up_recipe_behaviour
        (   local_peer_id : libp2p::PeerId
          , local_sender : mpsc::UnboundedSender<RecipeResponse>) -> RecipeBehaviour
{
  let mut behaviour = RecipeBehaviour {
      floodsub: Floodsub::new(local_peer_id.clone()),
      mdns: Mdns::new(Default::default())
          .await
          .expect("can create mdns"),
      local_sender,
      local_peer_id
  };

  // Subscribe our specific network behaviour to be subscribed to the "recipes" topic.
  behaviour.floodsub.subscribe(RECIPE_TOPIC.clone());
  behaviour
}

// Read local recipes and send back to the local channel that communicates with the p2p network
fn respond_with_recipes(local_sender: mpsc::UnboundedSender<RecipeResponse>, remote_peer_id: String) {
    // Spawn an awaited async function that will, when done, will send a recipe response.
    tokio::spawn(async move {
        match local_data::read_local_recipes().await {
            Ok(recipes) => {
                let resp = RecipeResponse {
                    transmit_type: TransmitType::ToAll,
                    receiver: remote_peer_id,
                    data: recipes.into_iter().collect(),
                };
                if let Err(e) = local_sender.send(resp) {
                    error!("error sending response via channel, {}", e);
                }
            }
            Err(e) => error!("error fetching local recipes to answer ALL request, {}", e),
        }
    });
}

// ## Commands from StdIn
//
pub async fn handle_command(line: String, swarm: &mut Swarm<RecipeBehaviour>) {
    match line.as_str() {
        cmd if cmd.starts_with("create r")
            => handle_create_recipe(cmd.strip_prefix("create r").expect("Can strip `create r`") ).await,
        cmd if cmd.starts_with("ls r")
            => handle_list_recipes().await,
        cmd if cmd.starts_with("req r")
                => handle_req_recipes(cmd.strip_prefix("req r").expect("Can strip req r"), swarm).await,
        _ => error!("unknown command"),
    }
}

//
// 1. `create r {recipeName}` creates a new recipe with the given name (and an incrementing id)
pub async fn handle_create_recipe(args: &str) {
    let elements: Vec<&str> = args.split("|").collect();
    if elements.len() < 1 {
        info!("too few arguments - Format: recipe_name");
    } else {
        let name = elements.get(0).expect("name is there");
        if let Err(e) = local_data::write_new_local_recipe(name).await {
            error!("error creating recipe: {}", e);
        };
    }
}
// 2. `ls r` lists recipes
pub async fn handle_list_recipes() {
    match local_data::read_local_recipes().await {
        Ok(v) => {
            info!("Local Recipes ({})", v.len());
            v.iter().for_each(|r| info!("{:?}", r));
        }
        Err(e) => error!("Command error: [ls r] couldn't read local recipes: {}", e),
    };
}

// 2. `req r ...` lists recipes
pub async fn handle_req_recipes(arg: &str, swarm: &mut Swarm<RecipeBehaviour>) {
    match arg {
        _ if arg.trim().is_empty() => {
            info!("Command error: [req r] missing an argument, specify \"all\" or \"<peer_id>\"")
        }
        // `req r all` send a request for all recipes from all known peers
        "all" => {
            let req = RecipeRequest {
                transmit_type: TransmitType::ToAll,
            };
            let json = serde_json::to_string(&req).expect("can jsonify request");
            swarm
                .behaviour_mut()
                .floodsub
                .publish(RECIPE_TOPIC.clone(), json.as_bytes());
        }
        // `req r <peerId>` sends a request for all recipes from a certain peer
        peer_id => {
            let req = RecipeRequest {
                transmit_type: TransmitType::ToOne(peer_id.to_owned()),
            };
            let json = serde_json::to_string(&req).expect("can jsonify request");
            swarm
                .behaviour_mut()
                .floodsub
                .publish(RECIPE_TOPIC.clone(), json.as_bytes());
        }
    };
}
