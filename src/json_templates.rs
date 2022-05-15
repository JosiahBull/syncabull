use serde::Deserialize;

#[derive(Deserialize)]
pub struct QueryData {
    pub code: String,
    pub state: String,
}
