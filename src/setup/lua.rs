use git2::{
    FetchOptions,
    build::{CheckoutBuilder, RepoBuilder},
};
use glob::{MatchOptions, Pattern, glob_with};
use mlua::{Error, ErrorContext, FromLua, Lua, Result, Table, UserData, UserDataRef, Value, Variadic};
use pathdiff::diff_paths;
use regex::{Captures, Regex};
use std::{
    cell::RefCell,
    collections::HashMap,
    fs::{exists, remove_dir_all},
    path::PathBuf,
    process::{Command, Stdio},
    rc::Rc,
};
use which::which;

use crate::cache::{FabricateCache, GitDependency};

struct FabricateAppData {
    source_dir: PathBuf,
    build_dir: PathBuf,
    builds: Rc<RefCell<Vec<Build>>>,
}

pub struct ConfigResult {
    install: HashMap<PathBuf, PathBuf>,
}

impl FromLua for ConfigResult {
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        let table = match value {
            Value::Nil => return Ok(ConfigResult { install: HashMap::new() }),
            Value::Table(table) => table,
            value => {
                return Err(Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: String::from("Config Result"),
                    message: None,
                });
            }
        };

        let mut install: HashMap<PathBuf, PathBuf> = HashMap::new();
        if table.contains_key("install")? {
            for (k, v) in table
                .get::<HashMap<PathBuf, UserDataRef<Artifact>>>("install")
                .context("invalid `install` on config result")?
                .into_iter()
            {
                install.insert(k, v.0.clone());
            }
        }

        Ok(ConfigResult { install })
    }
}

#[derive(Clone, Copy)]
pub enum DepStyle {
    Normal,
    Gcc,
    Msvc,
}

impl FromLua for DepStyle {
    fn from_lua(value: mlua::Value, _: &Lua) -> Result<Self> {
        if let Value::String(value) = &value {
            return match value.to_string_lossy().as_str() {
                "normal" => Ok(DepStyle::Normal),
                "gcc" => Ok(DepStyle::Gcc),
                "clang" => Ok(DepStyle::Gcc),
                "msvc" => Ok(DepStyle::Msvc),
                _ => Err(Error::runtime("depstyle has to be one of `normal`, `gcc`, `clang`, `msvc`")),
            };
        }

        Err(Error::FromLuaConversionError {
            from: value.type_name(),
            to: String::from("DepStyle"),
            message: None,
        })
    }
}

struct Executable(PathBuf);

impl UserData for Executable {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, exec| Ok(exec.0.file_name().unwrap().to_string_lossy().to_string()));
        fields.add_field_method_get("path", |_, exec| Ok(exec.0.clone()));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("invoke", |_, exec, args: Variadic<String>| {
            let output = Command::new(&exec.0).args(args).stdout(Stdio::piped()).stderr(Stdio::inherit()).output()?;

            if !output.status.success() {
                return Err(Error::runtime(format!("executable `{}` invocation failed", exec.0.to_string_lossy())));
            }

            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        });
    }
}

#[derive(Clone)]
pub struct Rule {
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub depstyle: DepStyle,
    pub build_compdb: bool,
    pub variables: Vec<String>,
}

impl UserData for Rule {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, rule| Ok(rule.name.clone()));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "build",
            |l, rule, (output, input, variables, implicit_inputs): (PathBuf, Vec<Value>, HashMap<String, String>, Option<Vec<Value>>)| {
                let appdata = l.app_data_ref::<FabricateAppData>().unwrap();

                if !output.is_relative() {
                    return Err(Error::runtime(format!("output must be a relative path `{}`", output.to_string_lossy().to_string())));
                }

                let output = output.normalize_lexically().map_err(|_| Error::runtime("output path cannot escape the build directory"))?;
                let output = PathBuf::from("output").join(path_to_file(output));

                let inputs_to_paths = |inputs: Vec<Value>| -> Result<Vec<PathBuf>> {
                    let mut paths: Vec<PathBuf> = Vec::new();

                    for input in inputs {
                        let userdata = match input.as_userdata() {
                            None => {
                                return Err(Error::FromLuaConversionError {
                                    from: input.type_name(),
                                    to: String::from("Source or Artifact"),
                                    message: None,
                                });
                            }
                            Some(userdata) => userdata,
                        };

                        let mut path: Option<PathBuf> = None;

                        if userdata.is::<Source>() {
                            path = Some(userdata.borrow::<Source>()?.0.clone());
                        }

                        if userdata.is::<Artifact>() {
                            path = Some(userdata.borrow::<Artifact>()?.0.clone());
                        }

                        let path = match path {
                            None => {
                                return Err(Error::FromLuaConversionError {
                                    from: input.type_name(),
                                    to: String::from("Source or Artifact"),
                                    message: None,
                                });
                            }
                            Some(path) => appdata.source_dir.join(path),
                        };

                        let path = match diff_paths(&path, &appdata.build_dir) {
                            None => {
                                return Err(Error::runtime(format!(
                                    "failed to resolve relative path to build dir for input `{}`",
                                    path.to_string_lossy().to_string()
                                )));
                            }
                            Some(path) => path,
                        };

                        paths.push(path);
                    }

                    Ok(paths)
                };

                let mut final_variables = HashMap::new();
                for (key, mut value) in variables {
                    if RESERVED_VARIABLES.contains(&key.as_str()) {
                        return Err(Error::runtime(format!("variables contains a reserved variable `{}`", key)));
                    }

                    if BUILTIN_VARIABLES.contains(&key.as_str()) {
                        if key == "depfile" {
                            let path = PathBuf::from(value);
                            if !path.is_relative() {
                                return Err(Error::runtime(format!("depfile must be a relative path `{}`", path.to_string_lossy().to_string())));
                            }

                            let path = path.normalize_lexically().map_err(|_| Error::runtime("depfile path cannot escape the build directory"))?;

                            value = PathBuf::from("output/depfiles").join(path_to_file(path)).to_string_lossy().to_string();
                        }

                        final_variables.insert(key, value);
                        continue;
                    }

                    if !rule.variables.contains(&key) {
                        return Err(Error::runtime(format!("variables contains an unknown variable `{}`", key)));
                    }

                    final_variables.insert(format!("fabvar_{}", key), value);
                }

                let mut build = Build {
                    rule: rule.name.clone(),
                    output: output.clone(),
                    input: inputs_to_paths(input)?,
                    implicit_inputs: None,
                    variables: final_variables,
                };

                if let Some(implicit_inputs) = implicit_inputs {
                    build.implicit_inputs = Some(inputs_to_paths(implicit_inputs)?);
                }

                appdata.builds.borrow_mut().push(build);

                Ok(Artifact(output))
            },
        );
    }
}

struct Source(PathBuf);

impl UserData for Source {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("rel_path", |_, source| Ok(source.0.clone()));
        fields.add_field_method_get("abs_path", |l, source| Ok(l.app_data_ref::<FabricateAppData>().unwrap().source_dir.join(&source.0)));
    }
}

struct Artifact(PathBuf);

impl UserData for Artifact {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("rel_path", |_, artifact| Ok(artifact.0.clone()));
        fields.add_field_method_get("abs_path", |l, artifact| Ok(l.app_data_ref::<FabricateAppData>().unwrap().source_dir.join(&artifact.0)));
    }
}

pub struct Build {
    pub rule: String,
    pub output: PathBuf,
    pub input: Vec<PathBuf>,
    pub implicit_inputs: Option<Vec<PathBuf>>,
    pub variables: HashMap<String, String>,
}

const BUILTIN_VARIABLES: &'static [&'static str] = &["depfile"];
const RESERVED_VARIABLES: &'static [&'static str] = &["in", "out"];

pub fn lua_eval_config(
    source_dir: PathBuf,
    build_dir: PathBuf,
    config_path: PathBuf,
    options: HashMap<String, String>,
    cache: Option<FabricateCache>,
    dependency_overrides: HashMap<String, String>,
) -> Result<(Vec<Rule>, Vec<Build>, Vec<GitDependency>, HashMap<PathBuf, PathBuf>)> {
    let lua = Lua::new();

    let rules: Rc<RefCell<Vec<Rule>>> = Rc::new(RefCell::new(Vec::new()));
    let builds: Rc<RefCell<Vec<Build>>> = Rc::new(RefCell::new(Vec::new()));
    let git_deps: Rc<RefCell<Vec<GitDependency>>> = Rc::new(RefCell::new(Vec::new()));

    lua.set_app_data(FabricateAppData {
        source_dir: source_dir.clone(),
        build_dir: build_dir.clone(),
        builds: builds.clone(),
    });

    lua.load(include_str!("lua/generic.lua")).set_name("=fab_generic").exec()?;
    lua.load(include_str!("lua/builtins.lua")).set_name("=fab_builtins").exec()?;

    let fab_table = lua.create_table()?;
    fab_table.set(
        "path_join",
        lua.create_function(|l, components: Variadic<PathBuf>| {
            let mut path = PathBuf::new();
            for component in components {
                path = path.join(component);
            }
            Ok(Value::String(l.create_string(path.to_string_lossy().to_string())?))
        })?,
    )?;
    fab_table.set("source_dir", {
        let source_dir = source_dir.clone();
        lua.create_function(move |l, ()| Ok(Value::String(l.create_string(source_dir.to_string_lossy().to_string())?)))?
    })?;
    fab_table.set("build_dir", {
        let build_dir = build_dir.clone();
        lua.create_function(move |l, ()| Ok(Value::String(l.create_string(build_dir.to_string_lossy().to_string())?)))?
    })?;
    fab_table.set(
        "typeof",
        lua.create_function(|_, v: Value| match v {
            Value::UserData(userdata) => {
                if userdata.is::<Executable>() {
                    return Ok("executable");
                }

                if userdata.is::<Source>() {
                    return Ok("source");
                }

                if userdata.is::<Rule>() {
                    return Ok("rule");
                }

                if userdata.is::<Artifact>() {
                    return Ok("artifact");
                }

                Ok("unknown")
            }
            _ => Err(Error::runtime("not userdata")),
        })?,
    )?;
    fab_table.set(
        "which",
        lua.create_function(|_, lookup: String| match which(lookup) {
            Err(which::Error::CannotFindBinaryPath) => Ok(None),
            Err(err) => Err(Error::runtime(err)),
            Ok(path) => Ok(Some(Executable(path))),
        })?,
    )?;
    fab_table.set(
        "option",
        lua.create_function(move |l, (name, option_type, optional): (String, Value, bool)| {
            if !name.chars().all(|c: char| c.is_alphabetic() || c == '-' || c == '_') {
                return Err(Error::runtime(format!("option name `{}` contains invalid characters", name)));
            }

            let value = match options.get(&name) {
                None => {
                    if optional {
                        return Ok(Value::Nil);
                    }
                    return Err(Error::runtime(format!("option `{}` is missing", name)));
                }
                Some(value) => value,
            };

            let error = Error::FromLuaConversionError {
                from: option_type.type_name(),
                to: String::from("Option Type"),
                message: Some(String::from("option type can only be \"string\", \"number\", \"boolean\", or a list of valid string values")),
            };

            match option_type {
                Value::String(str) => match str.to_string_lossy().as_str() {
                    "string" => Ok(Value::String(l.create_string(value)?)),
                    "number" => Ok(Value::Number(
                        value.parse::<f64>().map_err(|_| Error::runtime(format!("value `{}` for option `{}` is not a number", value, name)))?,
                    )),
                    "boolean" => {
                        let value = match value.as_str() {
                            "ok" | "yes" | "true" => true,
                            "no" | "false" => false,
                            _ => return Err(Error::runtime(format!("value `{}` for option `{}` is not a boolean", value, name))),
                        };
                        Ok(Value::Boolean(value))
                    }
                    _ => return Err(error),
                },
                Value::Table(table) => {
                    for pair in table.pairs::<Value, Value>() {
                        let (_, v) = pair?;
                        let str = match v.as_string() {
                            None => return Err(error),
                            Some(v) => v.to_string_lossy().to_string(),
                        };

                        if *value == str {
                            return Ok(v);
                        }
                    }
                    return Err(Error::runtime(format!("value `{}` for option `{}` is not a valid", value, name)));
                }
                _ => Err(error),
            }
        })?,
    )?;
    fab_table.set("git", {
        let git_deps_store = Rc::clone(&git_deps);
        lua.create_function(move |_, (name, url, revision): (String, String, String)| {
            if !name.chars().all(|c: char| c.is_alphabetic() || c == '-' || c == '_' || c == '.') {
                return Err(Error::runtime(format!("git dependency name `{}` contains invalid characters", name)));
            }

            let mut git_deps = git_deps_store.borrow_mut();

            if git_deps.iter().find(|v| v.name == name).is_some() {
                return Err(Error::runtime(format!("git dependency defined twice `{}`", name)));
            }

            if let Some(dep_override) = dependency_overrides.get(&name) {
                git_deps.push(GitDependency { name, url, revision });
                return Ok(Artifact(PathBuf::from(dep_override)));
            }

            let build_relative_path = PathBuf::from("git").join(&name);
            let repo_path = build_dir.join(&build_relative_path);

            if exists(&repo_path)?
                && let Some(cache) = &cache
                && let Some(dep) = cache.git_dependencies.iter().find(|v| v.name == name)
            {
                if dep.url == url && dep.revision == revision {
                    git_deps.push(GitDependency { name, url, revision });
                    return Ok(Artifact(build_relative_path));
                }

                println!("Git dependency `{}` outdated, updating...", name);
            } else {
                println!("Git dependency `{}` not found, cloning...", name);
            }

            if exists(&repo_path)? {
                remove_dir_all(&repo_path)?;
            }

            let fetch_opts = FetchOptions::new();

            let mut builder = RepoBuilder::new();
            builder.fetch_options(fetch_opts);

            let repo = builder
                .clone(url.as_str(), repo_path.as_ref())
                .map_err(|err| Error::runtime(format!("Git clone failed for git dependency `{}`: {}", name, err)))?;

            let obj = repo
                .revparse_single(revision.as_str())
                .map_err(|err| Error::runtime(format!("Failed to resolve revision for git dependency `{}`: {}", name, err)))?;
            let commit = obj
                .peel_to_commit()
                .map_err(|err| Error::runtime(format!("Failed to resolve commit of revision for git dependency `{}`: {}", name, err)))?;

            repo.checkout_tree(commit.as_object(), Some(CheckoutBuilder::new().force()))
                .map_err(|err| Error::runtime(format!("Failed to checkout revision for git dependency `{}`: {}", name, err)))?;

            repo.set_head_detached(commit.id())
                .map_err(|err| Error::runtime(format!("Failed to set head for git dependency `{}`: {}", name, err)))?;

            git_deps.push(GitDependency { name, url, revision });

            Ok(Artifact(build_relative_path))
        })?
    })?;
    fab_table.set(
        "glob",
        lua.create_function(|lua, (pattern, opts): (String, Option<Table>)| {
            let opts = match opts {
                Some(table) => table,
                None => lua.create_table()?,
            };

            let mut match_options = MatchOptions::new();
            if let Some(case_sensitive) = opts.get::<Option<bool>>("case_sensitive").context("case_sensitive must be a boolean")? {
                match_options.case_sensitive = case_sensitive;
            }
            if let Some(require_literal_separator) = opts.get::<Option<bool>>("require_literal_separator").context("require_literal_separator must be a boolean")? {
                match_options.require_literal_separator = require_literal_separator;
            }
            if let Some(require_literal_leading_dot) = opts.get::<Option<bool>>("require_literal_leading_dot").context("require_literal_leading_dot must be a boolean")? {
                match_options.require_literal_leading_dot = require_literal_leading_dot;
            }

            let paths = glob_with(pattern.as_str(), match_options).map_err(|err| Error::runtime(format!("invalid glob pattern: {}", err)))?;

            let mut exclude_patterns = Vec::new();
            if let Some(excludes) = opts.get::<Option<Table>>("excludes").context("excludes must be a table of strings")? {
                for pair in excludes.pairs::<Value, Value>() {
                    let (_, value): (_, Value) = pair?;
                    let str = match value.as_string() {
                        None => return Err(Error::runtime("excludes must only contain strings")),
                        Some(v) => v.to_string_lossy().to_string(),
                    };

                    let pattern = Pattern::new(str.as_str()).map_err(|err| Error::runtime(format!("invalid exclude glob pattern: {}", err)))?;
                    exclude_patterns.push(pattern);
                }
            }

            let matched_paths = lua.create_table()?;
            let mut idx = 1;

            'entries: for entry in paths {
                let path = entry.map_err(|err| Error::runtime(format!("failed to read glob entry: {}", err)))?;
                let path_str = path.to_string_lossy().to_string();

                for pattern in &exclude_patterns {
                    if pattern.matches_path_with(path.as_path(), match_options) {
                        continue 'entries;
                    }
                }

                matched_paths.set(idx, path_str)?;
                idx += 1;
            }

            Ok(matched_paths)
        })?,
    )?;
    fab_table.set(
        "def_source",
        lua.create_function(move |_, str: String| {
            let full_path = PathBuf::from(&str)
                .canonicalize()
                .map_err(|err| Error::runtime(format!("failed to resolve source path `{}`: {}", str, err)))?;

            let relative_path = full_path.strip_prefix(&source_dir).map_err(|_| {
                Error::runtime(format!(
                    "source path `{}` is not within the source directory `{}`",
                    full_path.to_string_lossy(),
                    source_dir.to_string_lossy()
                ))
            })?;

            Ok(Source(relative_path.to_path_buf()))
        })?,
    )?;
    fab_table.set("def_rule", {
        let rule_store = Rc::clone(&rules);
        lua.create_function(move |_, (name, command, description, depstyle, build_compdb): (String, String, _, _, _)| {
            if name.is_empty() {
                return Err(Error::runtime("empty rule name"));
            }

            if command.is_empty() {
                return Err(Error::runtime("empty rule command"));
            }

            let variables = RefCell::new(Vec::new());
            let var_parse = |capture: &Captures| {
                let (_, [var]) = capture.extract();

                let var = var.to_lowercase();

                if RESERVED_VARIABLES.contains(&var.as_str()) || BUILTIN_VARIABLES.contains(&var.as_str()) {
                    return format!("${}", var);
                }

                variables.borrow_mut().push(var.clone());

                return format!("$fabvar_{}", var);
            };

            let var_regex = Regex::new(r"@(.+?)@").map_err(|err| Error::runtime(err))?;
            let command = var_regex.replace_all(&command, &var_parse).to_string();

            let mut description: Option<String> = description;
            if let Some(desc) = description {
                description = Some(var_regex.replace_all(&desc.to_string(), &var_parse).to_string());
            }

            if name.starts_with("fab_") {
                return Err(Error::runtime("rule that begin with `fab_` are reserved"));
            }

            if !name.chars().all(|c: char| c.is_alphabetic() || c == '-' || c == '_') {
                return Err(Error::runtime(format!("rule name `{}` contains invalid characters", name)));
            }

            let rule = Rule {
                name,
                command,
                description,
                depstyle,
                build_compdb,
                variables: variables.into_inner(),
            };

            rule_store.borrow_mut().push(rule.clone());

            Ok(rule)
        })?
    })?;

    lua.globals().set("fab", fab_table)?;

    let result = lua.load(config_path).eval::<ConfigResult>()?;

    drop(lua);

    let rules = Rc::try_unwrap(rules).map_err(|_| Error::runtime("failed to collect rules"))?.into_inner();
    let builds = Rc::try_unwrap(builds).map_err(|_| Error::runtime("failed to collect builds"))?.into_inner();
    let git_deps = Rc::try_unwrap(git_deps).map_err(|_| Error::runtime("failed to collect git_deps"))?.into_inner();

    Ok((rules, builds, git_deps, result.install))
}

fn path_to_file(path: PathBuf) -> String {
    let mut components = Vec::new();
    for component in path.components() {
        components.push(component.as_os_str().to_string_lossy().to_string().replace("_", "__"));
    }
    return components.join("_");
}
