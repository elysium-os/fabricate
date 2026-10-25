use std::fs::create_dir_all;

use anyhow::{Context, Result};
use log::debug;

use crate::{
    args::{FabricateOptions, MainCommand},
    dependencies::{fetch_dependencies, read_dependencies_file},
};

mod install;
mod setup;
mod support;

pub fn run(opts: FabricateOptions) -> Result<()> {
    match opts.command {
        MainCommand::Fetch { project_dir, deps_config } => {
            let project_dir = project_dir
                .canonicalize()
                .context("Failed to canonicalize project directory path (it likely does not exist)")?;

            debug!("Project directory: {}", project_dir.display());

            let deps_config_path = project_dir
                .join(deps_config)
                .canonicalize()
                .context("Failed to canonicalize dependency config file path (it likely does not exist)")?;

            debug!("Dependency config file: {}", deps_config_path.display());

            eprintln!("Fetching dependencies...");
            let deps = read_dependencies_file(&deps_config_path).context("Failed to read dependency config")?;

            let deps_dir = project_dir.join("fab-deps");
            create_dir_all(&deps_dir).context("Failed to create dependencies directory")?;

            let _ = fetch_dependencies(&deps_dir, true, &deps).context("Failed to fetch dependencies")?;

            eprintln!("Dependencies are up-to-date");

            Ok(())
        }
        MainCommand::Setup(args) => setup::run(args),
        MainCommand::Support { command } => support::run(command),
        MainCommand::Install(args) => install::run(args),
    }
}
