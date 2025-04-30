use std::{
    collections::HashMap,
    env,
    fmt::Display,
    fs::{create_dir_all, read, write},
    io::{self},
    path::{Path, PathBuf},
};

use clap::{Args, Parser, Subcommand};
use colog::format::CologStyle;
use compiler::Compiler;
use dependency::Dependency;
use executable::Executable;
use glob::glob;
use include_dir::IncludeDirectory;
use json::JsonValue;
use linker::Linker;
use log::warn;
use mlua::{IntoLua, Lua, Value, Variadic};
use ninja_writer::Ninja;
use source::Source;
use thiserror::Error;

mod compiler;
mod dependency;
mod executable;
mod include_dir;
mod linker;
mod object;
mod source;

#[derive(Parser)]
#[command(version, next_line_help = true)]
struct FabOptions {
    #[arg(long, default = "fab.lua", help = "config file path")]
    config: String,

    #[command(subcommand)]
    command: MainCommand,
}

#[derive(Subcommand)]
enum MainCommand {
    #[command(about = "configure build")]
    Configure(ConfigureOptions),
}

#[derive(Args)]
struct ConfigureOptions {
    #[arg(short = 'D', long = "option", help = "pass user defined option", value_parser = keyvalue_opt_validate)]
    options: Vec<(String, String)>,

    #[arg(long = "prefix", help = "prefix", default_value = "/usr/local")]
    prefix: String,

    #[arg(help = "path to build directory", default_value = "build")]
    builddir: String,
}

#[derive(Error, Debug)]
enum FabError {
    Io(#[from] io::Error),
    Lua(#[from] mlua::Error),
    Toml(#[from] toml::ser::Error),
}

struct FabContext {
    options: HashMap<String, String>,
    project_root: PathBuf,
    build_cache: PathBuf,

    ninja: Ninja,
    compile_commands: Vec<JsonValue>,

    rule_cache: Vec<String>,
    dependency_cache: Vec<String>,
}

struct FabLogStyle;

impl CologStyle for FabLogStyle {
    fn prefix_token(&self, level: &log::Level) -> String {
        format!("{} >> ", self.level_color(level, self.level_token(level)))
    }

    fn level_token(&self, level: &log::Level) -> &str {
        match *level {
            log::Level::Error => "Error",
            log::Level::Warn => "Warning",
            log::Level::Info => "Info",
            log::Level::Debug => "Debug",
            log::Level::Trace => "Trace",
        }
    }
}

impl Display for FabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FabError::Io(err) => write!(f, "{}", err),
            FabError::Toml(err) => write!(f, "{}", err),
            FabError::Lua(err) => write!(f, "{}", err),
        }
    }
}

const BUILTIN: &'static str = include_str!("lua/builtin.lua");

fn keyvalue_opt_validate(s: &str) -> Result<(String, String), String> {
    match s.split_once("=") {
        None => Err(format!("`{s}` is not a key value pair")),
        Some((key, value)) => Ok((key.to_string(), value.to_string())),
    }
}

impl FabContext {
    fn path_dependencies(&self) -> PathBuf {
        self.build_cache.join("dependencies")
    }
}

fn main() {
    colog::default_builder().format(colog::formatter(FabLogStyle)).init();

    if let Err(err) = run_main() {
        eprintln!("{}", err);
    }
}

fn run_main() -> Result<(), FabError> {
    let options = FabOptions::parse();

    match options.command {
        MainCommand::Configure(configure_options) => {
            let config_path = Path::new(&options.config).canonicalize()?;

            create_dir_all(&configure_options.builddir)?;
            let build_cache = Path::new(&configure_options.builddir).canonicalize()?;

            let project_root = match config_path.parent() {
                None => panic!("Failed to resolve config directory"),
                Some(config_dir) => {
                    env::set_current_dir(config_dir)?;
                    config_dir.to_owned()
                }
            };

            let lua = Lua::new();

            // Setup fab table
            let fab_table = lua.create_table()?;

            fab_table.set("find_executable", lua.create_function(Executable::find)?)?;

            fab_table.set(
                "glob",
                lua.create_function(|_: &Lua, pattern: String| {
                    let mut paths: Vec<PathBuf> = Vec::new();
                    for entry in glob(&pattern).unwrap_or_else(|err| panic!("Glob pattern `{}` failed: {}", pattern, err)) {
                        match entry {
                            Ok(path) => paths.push(path),
                            Err(_) => continue,
                        }
                    }
                    Ok(paths)
                })?,
            )?;

            fab_table.set(
                "option",
                lua.create_function(|lua: &Lua, (name, default): (String, Value)| {
                    let fab_context = lua.app_data_ref::<FabContext>().unwrap();

                    if !fab_context.options.contains_key(&name) {
                        return Ok(default);
                    }

                    Ok(fab_context.options[&name].clone().into_lua(lua)?)
                })?,
            )?;

            fab_table.set(
                "project_root",
                lua.create_function(|lua: &Lua, _: ()| Ok(lua.app_data_ref::<FabContext>().unwrap().project_root.clone()))?,
            )?;

            fab_table.set("create_compiler", lua.create_function(Compiler::create)?)?;
            fab_table.set("create_linker", lua.create_function(Linker::create)?)?;
            fab_table.set("dependency", lua.create_function(Dependency::create)?)?;
            fab_table.set("source", lua.create_function(Source::create)?)?;
            fab_table.set("include_directory", lua.create_function(IncludeDirectory::create)?)?;

            // Globals
            lua.globals().set("fab", fab_table)?;

            lua.globals().set(
                "path",
                lua.create_function(|_: &Lua, (base, parts): (PathBuf, Variadic<PathBuf>)| {
                    let mut path = base;
                    for part in parts {
                        path = path.join(part);
                    }
                    Ok(path)
                })?,
            )?;

            lua.globals()
                .set("panic", lua.create_function(|_: &Lua, message: String| -> mlua::Result<()> { panic!("{}", message) })?)?;

            lua.globals().set(
                "warn",
                lua.create_function(|_: &Lua, message: String| {
                    warn!("{}", message);
                    Ok(())
                })?,
            )?;

            // Setup fab context
            let mut options: HashMap<String, String> = HashMap::new();
            for (key, value) in configure_options.options {
                options.insert(key, value);
            }

            let fab_context = FabContext {
                options,
                project_root,
                build_cache: build_cache.clone(),
                ninja: Ninja::new(),
                compile_commands: Vec::new(),
                rule_cache: Vec::new(),
                dependency_cache: Vec::new(),
            };

            // Create build cache
            create_dir_all(&build_cache)?;
            create_dir_all(&build_cache.join("objects"))?;
            create_dir_all(&build_cache.join("depfiles"))?;
            create_dir_all(fab_context.path_dependencies())?;

            write(build_cache.join(".gitignore"), "# Generated by Fab.\n*")?;

            // Execute build script
            lua.set_app_data(fab_context);
            lua.load(BUILTIN).set_name("=builtin").exec()?;
            lua.load(read(config_path)?).set_name(format!("@{}", &options.config)).exec()?;

            // Write ninja build
            let fab_context = lua.app_data_mut::<FabContext>().unwrap();
            let ninja_config = fab_context.ninja.to_string();
            write(build_cache.join("build.ninja"), ninja_config)?;
            write(build_cache.join("compile_commands.json"), JsonValue::Array(fab_context.compile_commands.clone()).pretty(4))?;

            let mut config_table = toml::Table::new();
            config_table.insert("prefix".to_string(), toml::Value::String(configure_options.prefix.clone()));
            config_table.insert("builddir".to_string(), toml::Value::String(configure_options.builddir.clone()));
            write(build_cache.join("fab.config.toml"), toml::to_string(&config_table)?)?;
        }
    }

    Ok(())
}
