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
use tokio::fs;

const LOCAL_STORAGE_FILE_PATH: &str = "./recipes.json";

pub type Recipes = Vec<Recipe>;
#[derive(Debug, Serialize, Deserialize)]
pub struct Recipe {
    id : usize,
    name: String
}

// ## Auxiliary File IO
//
// reads all locally stored recipes
pub async fn read_local_recipes() -> Result<Recipes, Box<dyn std::error::Error>> {
  let content: Vec<u8> = fs::read(LOCAL_STORAGE_FILE_PATH).await?;
  let result: Recipes = serde_json::from_slice(&content)?;
  Ok(result)
}
// (over)writes all locally stored recipes
pub async fn write_local_recipes(recipes: &Recipes) -> Result<(), Box<dyn std::error::Error>> {
  let json = serde_json::to_string(&recipes)?;
  fs::write(LOCAL_STORAGE_FILE_PATH, &json).await?;
  Ok(())
}
// appends to the locally stored recipes
pub async fn write_new_local_recipe(recipe_name: &str) -> Result<(), Box<dyn std::error::Error>> {
  let mut local_recipes = read_local_recipes().await?;
  let new_id = match local_recipes.iter().max_by_key(|r| r.id) {
      Some(v) => v.id + 1,
      None => 0,
  };
  local_recipes.push(Recipe {
      id: new_id,
      name: recipe_name.to_owned()
  });
  write_local_recipes(&local_recipes).await?;

  info!("Created recipe:");
  info!("Name: {}", recipe_name);

  Ok(())
}

