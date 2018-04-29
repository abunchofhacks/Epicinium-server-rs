/* Order */

use logic::player::Player;


#[derive(Default, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct Vision(Vec<Player>);
