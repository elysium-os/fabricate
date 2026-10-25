use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, value_parser};
use clap_complete::Shell;

#[derive(Parser)]
#[command(version, next_line_help = true)]
pub struct FabricateOptions {
    #[command(subcommand)]
    pub command: MainCommand,
}

#[derive(Subcommand)]
pub enum MainCommand {
    #[command(about = "pre-fetch dependencies")]
    Fetch {
        #[arg(short, long, help = "path to project directory", env = "PROJECT_DIR", default_value = "./", global = true)]
        project_dir: PathBuf,

        #[arg(long, help = "path to dependency config file", default_value = "./fab-deps.toml", global = true)]
        deps_config: PathBuf,
    },

    #[command(about = "setup (configure) the build")]
    Setup(SetupArgs),

    #[command(about = "install built artifacts")]
    Install(InstallArgs),

    #[command(about = "miscellaneous support tooling")]
    Support {
        #[command(subcommand)]
        command: SupportCommand,
    },
}

#[derive(Args)]
pub struct SetupArgs {
    #[arg(
        short,
        long,
        help = "path to build directory",
        env = "BUILD_DIR",
        default_value = "./build",
        global = true
    )]
    pub build_dir: PathBuf,

    #[arg(short, long, help = "path to project directory", env = "PROJECT_DIR", default_value = "./", global = true)]
    pub project_dir: PathBuf,

    #[arg(long, help = "path to config file", default_value = "./fab.lua", global = true)]
    pub config: PathBuf,

    #[arg(long, help = "path to dependency config file", default_value = "./fab-deps.toml", global = true)]
    pub deps_config: PathBuf,

    #[arg(long, help = "installation prefix", default_value = "/usr/local", global = true)]
    pub prefix: String,

    #[arg(long = "option", short = 'o', value_parser = opt_keyvalue, help = "Specify the value of a *user defined* option in the format of key=value")]
    pub option: Vec<(String, String)>,
}

#[derive(Args)]
pub struct InstallArgs {
    #[arg(
        short,
        long,
        help = "path to build directory",
        env = "BUILD_DIR",
        default_value = "./build",
        global = true
    )]
    pub build_dir: PathBuf,

    #[arg(long, short = 'd', help = "specify the destdir of the install", env = "DESTDIR")]
    pub dest_dir: Option<String>,
}

#[derive(Subcommand)]
pub enum SupportCommand {
    #[command(about = "generate lua lsp configuration")]
    SetupLSP {
        #[arg(long, help = "directory to place lua bindings in", default_value_os_t = default_lsp_support_path())]
        support_dir: PathBuf,
    },

    #[command(about = "generate shell completions for fabricate")]
    Completions {
        #[arg(help = "shell to generate completions for", value_parser = value_parser!(Shell))]
        shell: Shell,
    },

    #[command(about = "print out license and third party notices")]
    Licenses,
}

fn default_lsp_support_path() -> PathBuf {
    dirs::data_dir()
        .map(|dir| dir.join("fabricate").join("lsp-support"))
        .unwrap_or(PathBuf::from(".fabricate-lsp-support"))
}

fn opt_keyvalue(s: &str) -> Result<(String, String), String> {
    match s.split_once("=") {
        None => Err(format!("`{s}` is not a key value pair")),
        Some((key, value)) => Ok((key.to_string(), value.to_string())),
    }
}
