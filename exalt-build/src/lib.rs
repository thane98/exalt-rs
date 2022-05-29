use exalt_lir::Game;
use serde::{Serialize, Deserialize};

use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildConfig {
    pub game: Game,
    pub output: String,
    pub sources: BuildSources,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildSources {
    pub src: String,
    pub headers: Vec<String>,
}

pub fn build(_config: &BuildConfig) -> Result<Vec<u8>> {
    todo!()
}
