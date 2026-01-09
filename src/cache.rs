use std::{
    collections::HashMap,
    fs::{exists, read_to_string, write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GitDependency {
    pub name: String,
    pub url: String,
    pub revision: String,
}

#[derive(Serialize, Deserialize)]
pub struct FabricateCache {
    #[serde(skip_serializing, skip_deserializing)]
    path: PathBuf,

    version: i64,

    pub prefix: String,
    pub options: HashMap<String, String>,
    pub installs: HashMap<PathBuf, PathBuf>,
    pub git_dependencies: Vec<GitDependency>,
}

const CURRENT_VERSION: i64 = 1;

fn get_version(cache_data: &String) -> Result<i64> {
    let cache = toml::from_str::<toml::Table>(cache_data).context("Failed to parse fabricate cache")?;

    match cache.get("version") {
        Some(toml::Value::Integer(version)) => Ok(*version),
        Some(_) => Ok(0),
        None => Ok(0),
    }
}

impl FabricateCache {
    pub fn new(path: &Path, prefix: String, options: HashMap<String, String>, installs: HashMap<PathBuf, PathBuf>, git_dependencies: Vec<GitDependency>) -> FabricateCache {
        FabricateCache {
            path: path.to_path_buf(),
            version: CURRENT_VERSION,
            prefix,
            options,
            installs,
            git_dependencies,
        }
    }

    pub fn load(path: &Path) -> Result<Option<FabricateCache>> {
        if exists(path)? {
            let cache_data = read_to_string(path).context("Failed to read fabricate cache")?;

            let version = get_version(&cache_data)?;
            if version == 0 {
                bail!("Fabricate cache is not intact");
            }

            if version > CURRENT_VERSION {
                bail!("Unsupported fabricate cache version: {}", version);
            }

            let mut cache: FabricateCache = toml::from_str(&cache_data).context("Failed to parse fabricate cache")?;
            cache.path = path.to_path_buf();
            return Ok(Some(cache));
        }

        Ok(None)
    }

    pub fn update(&self) -> Result<()> {
        write(&self.path, toml::to_string(self).context("Failed to serialize fabricate cache")?).context("Failed to write fabricate cache")?;
        Ok(())
    }
}
