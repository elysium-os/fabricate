use std::{collections::HashMap, path::PathBuf, sync::Arc};

pub mod lua;

#[derive(Debug)]
pub enum DepStyle {
    Normal,
    Gcc,
    Msvc { prefix: Option<String> },
}

#[derive(Debug)]
pub struct Rule {
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub depstyle: DepStyle,
    pub variables: Vec<String>,
    pub build_compdb: bool,
}

#[derive(Debug)]
pub struct Build {
    pub rule: Arc<Rule>,
    pub output: PathBuf,
    pub input: Vec<PathBuf>,
    pub implicit_inputs: Vec<PathBuf>,
    pub variables: HashMap<String, String>,
}

#[derive(Debug)]
pub struct Config {
    pub rules: Vec<Arc<Rule>>,
    pub builds: Vec<Build>,
    pub installs: HashMap<PathBuf, PathBuf>,
}
