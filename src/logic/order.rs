/* Order */

use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Order(serde_json::Value);
