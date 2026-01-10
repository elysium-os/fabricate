use std::{
    fs::{copy, create_dir_all},
    path::PathBuf,
    process::Command,
};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use which::which;

use crate::{cache::FabricateCache, setup::setup};

mod cache;
mod setup;

#[derive(Parser)]
#[command(version, next_line_help = true)]
struct FabricateOptions {
    #[arg(short, long, help = "path to build directory", default_value = "build")]
    build_dir: String,

    #[command(subcommand)]
    command: MainCommand,
}

#[derive(Subcommand)]
enum MainCommand {
    #[command()]
    Setup(SetupOpts),

    #[command()]
    Build,

    #[command()]
    Install(InstallOpts),
}

#[derive(Args)]
struct SetupOpts {
    #[arg(long, help = "Installation prefix (default: /usr)", default_value = "fab.lua")]
    prefix: String,

    #[arg(long, help = "Fabricate configuration file path (default: fab.lua)", default_value = "fab.lua")]
    config: String,

    #[arg(long = "option", short = 'o', value_parser = keyvalue_opt_validate, help = "Specify the value of a *user defined* option in the format of key=value")]
    option: Vec<(String, String)>,

    #[arg(long, value_parser = keyvalue_opt_validate, help = "Override a git dependency in the format of <dependency name>=<path>")]
    dependency_override: Vec<(String, String)>,
}

#[derive(Args)]
struct InstallOpts {
    #[arg(long, help = "Specify the destdir of the install", env = "DESTDIR")]
    dest_dir: Option<String>,
}

fn keyvalue_opt_validate(s: &str) -> Result<(String, String), String> {
    match s.split_once("=") {
        None => Err(format!("`{s}` is not a key value pair")),
        Some((key, value)) => Ok((key.to_string(), value.to_string())),
    }
}

fn main() -> Result<()> {
    let opts = FabricateOptions::parse();

    match opts.command {
        MainCommand::Setup(setup_opts) => setup(setup_opts.config, opts.build_dir, setup_opts.prefix, setup_opts.option, setup_opts.dependency_override)?,
        MainCommand::Build => {
            let ninja_path = which("ninja").context("Failed to locate ninja, cannot build")?;
            Command::new(ninja_path).arg("-C").arg(opts.build_dir).status()?;
        }
        MainCommand::Install(install_opts) => {
            let build_dir = PathBuf::from(opts.build_dir).canonicalize().context("Failed to resolve build directory path")?;

            let cache = match FabricateCache::load(&build_dir.join("fabricate_cache.toml"))? {
                None => bail!("Cache is not initialized, make sure the build directory path is correct"),
                Some(cache) => cache,
            };

            for (dest, src) in cache.installs {
                let abs_src = build_dir.join(&src);
                let mut abs_dest = PathBuf::from(&cache.prefix).join(&dest);

                if let Some(dest_dir) = &install_opts.dest_dir {
                    abs_dest = PathBuf::from(dest_dir).join(abs_dest)
                }

                if !abs_src.exists() {
                    bail!("Unable to install artifact `{}`, it does not exist", src.to_string_lossy());
                }

                if abs_src.is_dir() {
                    bail!("Unable to install artifact `{}`, it is a directory", src.to_string_lossy());
                }

                let dest_dir = match abs_dest.parent() {
                    None => bail!("Could not resolve directory path of destination"),
                    Some(path) => path,
                };

                create_dir_all(dest_dir).with_context(|| format!("Unable to install artifact `{}`, could not create dest dir", src.to_string_lossy()))?;

                copy(&src, abs_dest).with_context(|| format!("Unable to install artifact `{}`, could not copy", src.to_string_lossy()))?;
            }
        }
    }

    Ok(())
}
