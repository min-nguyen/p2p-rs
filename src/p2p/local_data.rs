use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::fs;

const LOCAL_STORAGE_FILE_PATH: &str = "./recipes.json";

pub type Recipes = Vec<Recipe>;
#[derive(Debug, Serialize, Deserialize)]
pub struct Recipe {
    id : usize,
    name: String
}

pub async fn handle_list_recipes() {
  match read_local_recipes().await {
      Ok(v) => {
          info!("Local Recipes ({})", v.len());
          v.iter().for_each(|r| info!("{:?}", r));
      }
      Err(e) => error!("error fetching local recipes: {}", e),
  };
}

pub async fn handle_create_recipe(cmd: &str) {
  let args: Option<&str>
      = cmd.strip_prefix("create")
           .map(|rest: &str| rest.trim());
  match args {
      Some("") | None => {
          info!("Command error: [create] missing an argument (name)")
      }
      // `req r all` send a request for all recipes from all known peers
      Some(name) => {
          if let Err(e) = write_new_local_recipe(name).await {
              error!("error creating recipe: {}", e);
          };
      }
  }
}

// ## Auxiliary File IO
// reads all locally stored recipes
async fn read_local_recipes() -> Result<Recipes, Box<dyn std::error::Error>> {
  let content: Vec<u8> = fs::read(LOCAL_STORAGE_FILE_PATH).await?;
  let result: Recipes = serde_json::from_slice(&content)?;
  Ok(result)
}
// (over)writes all locally stored recipes
async fn write_local_recipes(recipes: &Recipes) -> Result<(), Box<dyn std::error::Error>> {
  let json = serde_json::to_string(&recipes)?;
  fs::write(LOCAL_STORAGE_FILE_PATH, &json).await?;
  Ok(())
}
// appends to the locally stored recipes
async fn write_new_local_recipe(recipe_name: &str) -> Result<(), Box<dyn std::error::Error>> {
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

