use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NftMetadata {
    pub name: String,
    pub description: String,
    pub image: String,
    pub edition: u32,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Attribute {
    pub trait_type: String,
    pub value: String,
}
