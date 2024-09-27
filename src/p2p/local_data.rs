use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::fs;

/*
    *Local Data*: Defines the local data and IO for read and writing with it
*/

const LOCAL_STORAGE_FILE_PATH: &str = "./recipes.json";

pub type Recipes = Vec<Recipe>;
#[derive(Debug, Serialize, Deserialize)]
pub struct Recipe {
    id : usize,
    name: String
}


// reads all locally stored recipes
pub async fn read_local_recipes() -> Result<Recipes, Box<dyn std::error::Error>> {
  let content: Vec<u8> = fs::read(LOCAL_STORAGE_FILE_PATH).await?;
  let result: Recipes = serde_json::from_slice(&content)?;
  info!("local_data: Read recipe");
  Ok(result)
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

  info!("local_data: Created recipe:");
  info!("Name: {}", recipe_name);

  Ok(())
}
// (over)writes all locally stored recipes
async fn write_local_recipes(recipes: &Recipes) -> Result<(), Box<dyn std::error::Error>> {
  let json = serde_json::to_string(&recipes)?;
  fs::write(LOCAL_STORAGE_FILE_PATH, &json).await?;
  Ok(())
}
