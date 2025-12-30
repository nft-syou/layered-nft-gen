use serde::Deserialize;
use std::collections::HashMap;
use anyhow::Result;
use std::fs;

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let text = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&text)?;
        Ok(config)
    }
}


#[derive(Debug, Deserialize)]
pub struct Config {
    pub count: u32,
    pub output: OutputConfig,
    pub metadata: MetadataConfig,
    pub layers: Vec<LayerConfig>,
    pub constraints: Option<ConstraintsConfig>,
}

#[derive(Debug, Deserialize)]
pub struct OutputConfig {
    pub image_dir: String,
    pub metadata_dir: String,
    pub png_compression: Option<PngCompressionConfig>,
}

#[derive(Debug, Deserialize)]
pub struct PngCompressionConfig {
    pub enabled: bool,
    pub level: u8,
}

#[derive(Debug, Deserialize)]
pub struct MetadataConfig {
    pub base_image_url: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct LayerConfig {
    pub name: String,
    pub directory: String,
    pub rarity: Option<HashMap<String, f32>>,
}

#[derive(Debug, Deserialize)]
pub struct ConstraintsConfig {
    pub forbidden_pairs: Option<Vec<ForbiddenPair>>,
}

#[derive(Debug, Deserialize)]
pub struct ForbiddenPair {
    pub a: TraitValue,
    pub b: TraitValue,
}

#[derive(Debug, Deserialize)]
pub struct TraitValue {
    pub trait_type: String,
    pub value: String,
}