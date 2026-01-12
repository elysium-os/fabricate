use git2::{
    FetchOptions,
    build::{CheckoutBuilder, RepoBuilder},
};
use globset::{GlobBuilder, GlobSetBuilder};
use mlua::{Error, ErrorContext, FromLua, Lua, Result, Table, UserData, UserDataRef, Value, Variadic};
use pathdiff::diff_paths;
use regex::{Captures, Regex};
use std::{
    cell::RefCell,
    collections::HashMap,
    fs::{exists, remove_dir_all},
    path::PathBuf,
    rc::Rc,
};
use walkdir::WalkDir;
use which::which;

use crate::cache::{FabricateCache, GitDependency};

struct FabricateAppData {
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
        if let Value::Nil = value {
            return Ok(DepStyle::Normal);
        }

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
            |l, rule, (output, input, variables, implicit_inputs): (String, Vec<Value>, HashMap<String, String>, Option<Vec<Value>>)| {
                let appdata = l.app_data_ref::<FabricateAppData>().unwrap();

                if !output.chars().all(|c: char| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') {
                    return Err(Error::runtime(format!("output name `{}` contains invalid characters", output)));
                }

                let output = PathBuf::from("output").join(output);

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

                        if let Some(path) = path {
                            paths.push(path);
                            continue;
                        }

                        return Err(Error::FromLuaConversionError {
                            from: input.type_name(),
                            to: String::from("Source or Artifact"),
                            message: None,
                        });
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
                            if !value.chars().all(|c: char| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') {
                                return Err(Error::runtime(format!("depfile `{}` contains invalid characters", value)));
                            }

                            value = PathBuf::from("output").join(value).to_string_lossy().to_string();
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
        fields.add_field_method_get("path", |_, source| Ok(source.0.clone()));
    }
}

struct Artifact(PathBuf);

impl UserData for Artifact {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("path", |_, artifact| Ok(artifact.0.clone()));
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
    project_root: PathBuf,
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
    let git_overrides: Rc<HashMap<String, String>> = Rc::new(dependency_overrides);

    lua.set_app_data(FabricateAppData { builds: builds.clone() });

    lua.load(include_str!("lua/generic.lua")).set_name("=fab_generic").exec()?;

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
    fab_table.set("path_rel", {
        let project_root = project_root.clone();
        let build_dir = build_dir.clone();
        lua.create_function(move |_, mut path: PathBuf| {
            if path.is_relative() {
                path = project_root.join(path);
            }

            let path = match diff_paths(&path, &build_dir) {
                None => return Err(Error::runtime(format!("failed to resolve relative path `{}`", path.to_string_lossy()))),
                Some(path) => path,
            };

            Ok(path)
        })?
    })?;
    fab_table.set("project_dir", {
        let project_dir = project_root.clone();
        lua.create_function(move |l, ()| Ok(Value::String(l.create_string(project_dir.to_string_lossy().to_string())?)))?
    })?;
    fab_table.set("build_dir", {
        let build_dir = build_dir.clone();
        lua.create_function(move |l, ()| Ok(Value::String(l.create_string(build_dir.to_string_lossy().to_string())?)))?
    })?;
    fab_table.set(
        "typeof",
        lua.create_function(|_, v: Value| match v {
            Value::UserData(userdata) => {
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
        lua.create_function(move |_, lookup: String| match which(lookup) {
            Err(which::Error::CannotFindBinaryPath) => Ok(None),
            Err(err) => Err(Error::runtime(err)),
            Ok(path) => Ok(Some(path)),
        })?,
    )?;
    fab_table.set(
        "option",
        lua.create_function(move |l, (name, option_type, required): (String, Value, bool)| {
            if !name.chars().all(|c: char| c.is_alphabetic() || c == '-' || c == '_' || c == '.') {
                return Err(Error::runtime(format!("option name `{}` contains invalid characters", name)));
            }

            let value = match options.get(&name) {
                None => {
                    if required {
                        return Err(Error::runtime(format!("option `{}` is missing", name)));
                    }
                    return Ok(Value::Nil);
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
        let build_dir = build_dir.clone();
        let git_deps_store = Rc::clone(&git_deps);
        let git_overrides = Rc::clone(&git_overrides);
        lua.create_function(move |_, (name, url, revision): (String, String, String)| {
            if !name.chars().all(|c: char| c.is_alphabetic() || c == '-' || c == '_' || c == '.') {
                return Err(Error::runtime(format!("git dependency name `{}` contains invalid characters", name)));
            }

            let mut git_deps = git_deps_store.borrow_mut();

            if git_deps.iter().find(|v| v.name == name).is_some() {
                return Err(Error::runtime(format!("git dependency defined twice `{}`", name)));
            }

            if let Some(dep_override) = git_overrides.get(&name) {
                git_deps.push(GitDependency { name, url, revision });
                return Ok(Artifact(PathBuf::from(dep_override)));
            }

            let build_relative_path = PathBuf::from("git").join(&name);
            let repo_path = build_dir.join(&build_relative_path);

            if exists(&repo_path)? {
                if let Some(cache) = &cache {
                    if let Some(dep) = cache.git_dependencies.iter().find(|v| v.name == name) {
                        if dep.url == url && dep.revision == revision {
                            git_deps.push(GitDependency { name, url, revision });
                            return Ok(Artifact(build_relative_path));
                        }
                    }
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
    fab_table.set("glob", {
        let build_dir = build_dir.clone();
        let project_root = project_root.clone();
        lua.create_function(move |_, mut args: Variadic<Value>| {
            let opts: Option<Table> = match args.last() {
                Some(Value::Table(_)) => match args.pop() {
                    Some(Value::Table(t)) => Some(t),
                    _ => None,
                },
                _ => None,
            };

            if args.len() == 0 {
                return Err(Error::runtime("no globs in glob call"));
            }

            let mut case_sensitive = false;
            let mut require_literal_separator = false;
            let mut relative_to = project_root.to_path_buf();
            if let Some(opts) = opts {
                if let Some(value) = opts.get::<Option<bool>>("case_sensitive").context("case_sensitive must be a boolean")? {
                    case_sensitive = value;
                }

                if let Some(value) = opts.get::<Option<bool>>("require_literal_separator").context("require_literal_separator must be a boolean")? {
                    require_literal_separator = value;
                }

                if let Some(value) = opts.get::<Option<PathBuf>>("relative_to").context("relative_to must be a string")? {
                    if value.is_absolute() {
                        relative_to = value;
                    } else {
                        relative_to = build_dir.join(value);
                    }
                }
            }

            let mut positive_builder = GlobSetBuilder::new();
            let mut negative_builder = GlobSetBuilder::new();
            for arg in args {
                let mut pattern = match arg {
                    Value::String(str) => str.to_string_lossy(),
                    arg => {
                        return Err(Error::FromLuaConversionError {
                            from: arg.type_name(),
                            to: String::from("Glob"),
                            message: Some(String::from("Globs can only be strings")),
                        });
                    }
                };

                let mut builder = &mut positive_builder;
                if let Some(stripped) = pattern.strip_prefix("!") {
                    pattern = stripped.to_string();
                    builder = &mut negative_builder;
                }

                let glob = GlobBuilder::new(pattern.as_str())
                    .case_insensitive(!case_sensitive)
                    .literal_separator(require_literal_separator)
                    .build()
                    .map_err(|err| Error::runtime(format!("invalid glob: {}", err)))?;

                builder.add(glob);
            }

            let positive_set = positive_builder.build().map_err(|err| Error::runtime(format!("failed to build positive globset: {}", err)))?;
            let negative_set = negative_builder.build().map_err(|err| Error::runtime(format!("failed to build negative globset: {}", err)))?;

            let mut matches = Vec::new();
            for entry in WalkDir::new(&relative_to).follow_links(false).into_iter().filter_map(walkdir::Result::ok) {
                let path = entry.path();

                let rel = match path.strip_prefix(&relative_to) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                if positive_set.matches_all(rel) && !negative_set.is_match(rel) {
                    matches.push(path.to_path_buf());
                }
            }

            Ok(matches)
        })?
    })?;
    fab_table.set("def_source", {
        let git_overrides = Rc::clone(&git_overrides);
        lua.create_function(move |_, str: String| {
            let full_path = project_root
                .join(&str)
                .canonicalize()
                .map_err(|err| Error::runtime(format!("failed to resolve source path `{}`: {}", str, err)))?;

            let mut found_in_dep = false;
            for (_, v) in git_overrides.iter() {
                if !full_path.starts_with(v) {
                    continue;
                }

                found_in_dep = true;
                break;
            }

            if !found_in_dep && !full_path.starts_with(&project_root) {
                return Err(Error::runtime(format!(
                    "source `{}` is not within the project root `{}`",
                    full_path.to_string_lossy(),
                    project_root.to_string_lossy()
                )));
            }

            let path = match diff_paths(full_path, &build_dir) {
                None => {
                    return Err(Error::runtime(format!("failed to resolve relative path to build dir for source `{}`", &str)));
                }
                Some(path) => path,
            };

            Ok(Source(path))
        })?
    })?;
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

            if !name.chars().all(|c: char| c.is_alphabetic() || c == '-' || c == '_' || c == '.') {
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

    lua.load(include_str!("lua/builtins.lua")).set_name("=fab_builtins").exec()?;

    for m in [
        ("ld", include_str!("lua/modules/ld.lua")),
        ("lang_c", include_str!("lua/modules/lang_c.lua")),
        ("lang_nasm", include_str!("lua/modules/lang_nasm.lua")),
    ] {
        let name = m.0;
        let source = m.1.to_owned();

        let loader = lua.create_function(move |l, ()| l.load(&source).set_name(&format!("={}", name)).eval::<mlua::Value>())?;

        lua.preload_module(name, loader)?;
    }

    let result = lua.load(config_path).eval::<ConfigResult>()?;

    drop(lua);

    let rules = Rc::try_unwrap(rules).map_err(|_| Error::runtime("failed to collect rules"))?.into_inner();
    let builds = Rc::try_unwrap(builds).map_err(|_| Error::runtime("failed to collect builds"))?.into_inner();
    let git_deps = Rc::try_unwrap(git_deps).map_err(|_| Error::runtime("failed to collect git_deps"))?.into_inner();

    Ok((rules, builds, git_deps, result.install))
}
