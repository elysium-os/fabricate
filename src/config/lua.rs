use std::{
    collections::HashMap,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};

use globset::{GlobBuilder, GlobSetBuilder};
use include_dir::{Dir, include_dir};
use mlua::{
    AnyUserData, Either, Error, ErrorContext, Lua, LuaOptions, MetaMethod, MultiValue, StdLib, Table, UserData, UserDataFields, UserDataRef, Value,
    Variadic,
};
use mlua::{FromLua, Result};
use pathdiff::diff_paths;
use walkdir::WalkDir;
use which::which;

use crate::{
    config::{Build, Config, DepStyle, Rule},
    ninja::{BUILTIN_VARIABLES, RESERVED_VARIABLES},
};

pub const EMBEDDED_FILE_BUILTINS: &str = include_str!("./lua/builtins.lua");
pub const EMBEDDED_FILE_HELPERS: &str = include_str!("./lua/helpers.lua");
pub const EMBEDDED_FILE_META: &str = include_str!("./lua/meta.lua");

pub const EMBEDDED_DIR_MODULES: Dir = include_dir!("$CARGO_MANIFEST_DIR/modules");

struct FabricateAppData {
    build_dir: PathBuf,
    dependencies: HashMap<String, PathBuf>,
    rules: Vec<Arc<Rule>>,
    builds: Vec<Build>,
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
                "msvc" => Ok(DepStyle::Msvc { prefix: None }),
                value if let Some(prefix) = value.strip_prefix("msvc:") => Ok(DepStyle::Msvc {
                    prefix: Some(String::from(prefix)),
                }),
                _ => Err(Error::runtime("depstyle has to be one of `normal`, `gcc`, `msvc`, or `msvc:<prefix>`")),
            };
        }

        Err(Error::FromLuaConversionError {
            from: value.type_name(),
            to: String::from("DepStyle"),
            message: None,
        })
    }
}

struct RuleRef(Arc<Rule>);

impl UserData for RuleRef {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.0.name.clone()));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "build",
            |l, this, (output, input, variables, implicit_inputs): (String, Vec<AnyUserData>, HashMap<String, String>, Option<Vec<AnyUserData>>)| {
                if output.is_empty() {
                    return Err(Error::runtime("empty output name"));
                }

                if output.chars().any(|c| !c.is_alphanumeric() && !['-', '_', '.'].contains(&c)) {
                    return Err(Error::runtime(format!("output name `{}` contains invalid characters", output)));
                }

                let mut appdata = l.app_data_mut::<FabricateAppData>().unwrap();

                let output = PathBuf::from(output);
                if !output.is_relative() {
                    return Err(Error::runtime(format!("output path `{}` must be a relative path", output.display())));
                }

                let output = PathBuf::from("output").join(output);
                if appdata.builds.iter().any(|build| build.output == output) {
                    return Err(Error::runtime(format!("build with output path `{}` already exists", output.display())));
                }

                let inputs_to_paths = |inputs: Vec<AnyUserData>| -> Result<Vec<PathBuf>> {
                    let mut paths: Vec<PathBuf> = Vec::new();

                    for input in inputs {
                        let mut path: Option<PathBuf> = None;

                        if input.is::<Source>() {
                            let source = input.borrow::<Source>()?;
                            path = Some(source.0.clone());
                        }

                        if input.is::<Artifact>() {
                            let artifact = input.borrow::<Artifact>()?;
                            path = Some(artifact.0.clone());
                        }

                        if let Some(path) = path {
                            paths.push(path);
                            continue;
                        }

                        return Err(Error::FromLuaConversionError {
                            from: "Userdata",
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
                            if value.chars().any(|c| !c.is_alphanumeric() && !['-', '_', '.'].contains(&c)) {
                                return Err(Error::runtime(format!("depfile `{}` contains invalid characters", value)));
                            }

                            let path = PathBuf::from(value);
                            if !path.is_relative() {
                                return Err(Error::runtime(format!("depfile `{}` must be a relative path", path.display())));
                            }

                            value = PathBuf::from("output").join(path).to_string_lossy().to_string();
                        }

                        final_variables.insert(key, value);
                        continue;
                    }

                    if !this.0.variables.contains(&key) {
                        return Err(Error::runtime(format!("variables contains an unknown variable `{}`", key)));
                    }

                    final_variables.insert(format!("fabvar_{}", key), value);
                }

                let build = Build {
                    rule: this.0.clone(),
                    output: output.clone(),
                    input: inputs_to_paths(input)?,
                    implicit_inputs: inputs_to_paths(implicit_inputs.unwrap_or(Vec::new()))?,
                    variables: final_variables,
                };

                appdata.builds.push(build);

                Ok(Artifact(output))
            },
        );
    }
}

#[derive(Clone)]
struct FabPath(PathBuf);

impl FabPath {
    fn join(&self, other: &Path) -> Self {
        let mut joined = self.0.clone();
        for component in other.components() {
            match component {
                Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
                Component::ParentDir | Component::Normal(_) => joined.push(component),
            }
        }
        Self(joined)
    }
}

impl UserData for FabPath {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Div, |_, this, other: PathBuf| Ok(this.join(&other)));
        methods.add_meta_method(MetaMethod::ToString, |l, this, ()| {
            Ok(diff_paths(&this.0, &l.app_data_ref::<FabricateAppData>().unwrap().build_dir).expect("fab path is absolute"))
        });

        methods.add_method("join", |_, this, other: PathBuf| Ok(this.join(&other)));

        methods.add_method("glob", |_, this, mut args: Variadic<Value>| {
            let glob_options: Option<Table> = match args.last() {
                Some(Value::Table(_)) => match args.pop() {
                    Some(Value::Table(t)) => Some(t),
                    _ => None,
                },
                _ => None,
            };

            if args.len() == 0 {
                return Ok(Vec::new());
            }

            let mut case_sensitive = false;
            let mut require_literal_separator = false;
            if let Some(opts) = glob_options {
                if let Some(value) = opts.get::<Option<bool>>("case_sensitive").context("case_sensitive must be a boolean")? {
                    case_sensitive = value;
                }

                if let Some(value) = opts
                    .get::<Option<bool>>("require_literal_separator")
                    .context("require_literal_separator must be a boolean")?
                {
                    require_literal_separator = value;
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

            let positive_set = positive_builder
                .build()
                .map_err(|err| Error::runtime(format!("failed to build positive globset: {}", err)))?;
            let negative_set = negative_builder
                .build()
                .map_err(|err| Error::runtime(format!("failed to build negative globset: {}", err)))?;

            let mut matches = Vec::new();
            for entry in WalkDir::new(&this.0).follow_links(false).into_iter().filter_map(walkdir::Result::ok) {
                let path = entry.path();

                let rel = match path.strip_prefix(&this.0) {
                    Ok(r) => r,
                    Err(_) => continue,
                };

                if positive_set.matches_all(rel) && !negative_set.is_match(rel) {
                    matches.push(FabPath(path.to_path_buf()));
                }
            }

            Ok(matches)
        });
    }
}

struct Source(PathBuf);

impl UserData for Source {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("path", |_, this| Ok(this.0.clone()));
    }
}

struct Artifact(PathBuf);

impl UserData for Artifact {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("path", |_, this| Ok(this.0.clone()));
    }
}

struct Binary(PathBuf);

impl UserData for Binary {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("path", |_, this| Ok(this.0.clone()));
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| Ok(this.0.clone()));
    }
}

pub fn eval(
    project_dir: &Path,
    build_dir: &Path,
    config_path: &Path,
    dependencies: HashMap<String, PathBuf>,
    options: HashMap<String, String>,
) -> Result<Config> {
    let lua = Lua::new_with(StdLib::MATH | StdLib::STRING | StdLib::TABLE | StdLib::PACKAGE, LuaOptions::new())?;

    let package: Table = lua.globals().get("package")?;
    package.set(
        "path",
        format!("{}/?.lua;{}/?/init.lua", project_dir.to_string_lossy(), project_dir.to_string_lossy()),
    )?;

    lua.set_app_data(FabricateAppData {
        build_dir: build_dir.to_path_buf(),
        dependencies,
        rules: Vec::new(),
        builds: Vec::new(),
    });

    lua.load(EMBEDDED_FILE_BUILTINS).set_name("=fabricate_builtins").exec()?;
    lua.load(EMBEDDED_FILE_HELPERS).set_name("=fabricate_helpers").exec()?;

    let fab_table = lua.create_table()?;
    fab_table.set("path_join", lua.create_function(|_, (a, b): (PathBuf, PathBuf)| Ok(a.join(b)))?)?;
    fab_table.set(
        "which",
        lua.create_function(move |_, lookup: String| match which(lookup) {
            Err(which::Error::CannotFindBinaryPath) => Ok(None),
            Err(err) => Err(Error::runtime(err)),
            Ok(path) => Ok(Some(Binary(path))),
        })?,
    )?;
    fab_table.set("project_dir", FabPath(project_dir.to_path_buf()))?;
    fab_table.set(
        "dependency",
        lua.create_function(|l, name: String| {
            let app_data = l.app_data_mut::<FabricateAppData>().unwrap();

            let dependency_path = match app_data.dependencies.get(&name) {
                Some(path) => path,
                None => return Err(Error::runtime(format!("unknown dependency `{}`", name))),
            };

            Ok(FabPath(dependency_path.to_path_buf()))
        })?,
    )?;
    fab_table.set("resolve_source", {
        let build_dir = build_dir.to_path_buf();
        lua.create_function(move |_, path: UserDataRef<FabPath>| {
            let absolute_path = path.0.canonicalize().map_err(|err| match err {
                err if err.kind() == ErrorKind::NotFound => Error::runtime(format!("source path `{}` not found", path.0.display())),
                _ => Error::runtime(format!("failed to resolve source path `{}`: {}", path.0.display(), err)),
            })?;

            Ok(Source(diff_paths(&absolute_path, &build_dir).expect("source path is absolute")))
        })?
    })?;
    fab_table.set(
        "define_rule",
        lua.create_function(
            |l, (name, command, description, depstyle, build_compdb): (String, String, Option<String>, DepStyle, bool)| {
                if name.is_empty() {
                    return Err(Error::runtime("empty rule name"));
                }

                if name.starts_with("fab_") {
                    return Err(Error::runtime("rule that begin with `fab_` are reserved"));
                }

                if name.chars().any(|c: char| !c.is_alphanumeric() && !['-', '_', '.'].contains(&c)) {
                    return Err(Error::runtime(format!("rule name `{}` contains invalid characters", name)));
                }

                if command.is_empty() {
                    return Err(Error::runtime("empty rule command"));
                }

                let mut app_data = l.app_data_mut::<FabricateAppData>().unwrap();

                if app_data.rules.iter().any(|rule| rule.name == name) {
                    return Err(Error::runtime(format!("a rule with the name `{}` is already defined", name)));
                }

                let mut variables = Vec::new();
                let mut substitute_vars = |input: &str| -> String {
                    let mut out = String::with_capacity(input.len());

                    let mut rest = input;
                    while let Some(start) = rest.find('@') {
                        out.push_str(&rest[..start]);
                        let after = &rest[start + 1..];

                        let ident_len = after.find(|c: char| !(c == '_' || c.is_ascii_alphanumeric())).unwrap_or(after.len());
                        if after[ident_len..].starts_with('@') {
                            let ident = &after[..ident_len];
                            if ident.is_empty() {
                                out.push('@');
                            } else {
                                let name = ident.to_lowercase();

                                let resolved_var = if RESERVED_VARIABLES.contains(&name.as_str()) || BUILTIN_VARIABLES.contains(&name.as_str()) {
                                    format!("${}", name)
                                } else {
                                    variables.push(name.clone());
                                    format!("$fabvar_{}", name)
                                };

                                out.push_str(&resolved_var);
                            }
                            rest = &after[ident_len + 1..];
                        } else {
                            out.push('@');
                            rest = after;
                        }
                    }
                    out.push_str(rest);

                    out
                };

                let command = substitute_vars(&command);
                let mut description: Option<String> = description;
                if let Some(desc) = description {
                    description = Some(substitute_vars(&desc));
                }

                let rule = Arc::new(Rule {
                    name,
                    command,
                    description,
                    depstyle,
                    build_compdb,
                    variables,
                });

                app_data.rules.push(rule.clone());

                Ok(RuleRef(rule))
            },
        )?,
    )?;
    fab_table.set(
        "option",
        lua.create_function(move |l, (name, option_type, required): (String, Value, bool)| {
            if name.chars().any(|c: char| !c.is_alphanumeric() && !['-', '_', '.'].contains(&c)) {
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

            match option_type {
                Value::String(str) if str == "string" => Ok(Value::String(l.create_string(value)?)),
                Value::String(str) if str == "number" => {
                    Ok(Value::Number(value.parse::<f64>().map_err(|_| {
                        Error::runtime(format!("value `{}` for option `{}` is not a number", value, name))
                    })?))
                }
                Value::String(str) if str == "boolean" => {
                    let value = match value.as_str() {
                        "ok" | "yes" | "true" => true,
                        "no" | "false" => false,
                        _ => return Err(Error::runtime(format!("value `{}` for option `{}` is not a boolean", value, name))),
                    };
                    Ok(Value::Boolean(value))
                }
                Value::Table(table) => {
                    for pair in table.pairs::<Value, Value>() {
                        let (_, v) = pair?;
                        let str = match v.as_string() {
                            None => return Err(Error::runtime("the list of values must consist only of strings")),
                            Some(v) => v.to_string_lossy().to_string(),
                        };

                        if *value == str {
                            return Ok(v);
                        }
                    }
                    return Err(Error::runtime(format!("value `{}` for option `{}` is not a valid", value, name)));
                }
                _ => Err(Error::FromLuaConversionError {
                    from: option_type.type_name(),
                    to: String::from("Option Type"),
                    message: Some(String::from(
                        "option type can only be \"string\", \"number\", \"boolean\", or a list of valid string values",
                    )),
                }),
            }
        })?,
    )?;

    let globals = lua.globals();
    globals.set("fab", fab_table)?;
    globals.set(
        "print",
        lua.create_function(|_, args: MultiValue| {
            let line = args.iter().map(|v| v.to_string()).collect::<Result<Vec<_>>>()?.join("\t");
            eprintln!("[LUA PRINT] {line}");
            Ok(())
        })?,
    )?;

    lua.set_warning_function({
        let enabled = RwLock::new(true);
        let current_message: Mutex<Option<String>> = Mutex::new(None);
        move |_, message, continues| {
            match message.strip_prefix("@") {
                Some("on") => *enabled.write().unwrap() = true,
                Some("off") => *enabled.write().unwrap() = false,
                Some(_) => {}
                None => {
                    let mut current_message = current_message.lock().unwrap();
                    if *enabled.read().unwrap() {
                        let message = match current_message.as_deref() {
                            None => String::from(message),
                            Some(last) => format!("{} {}", last, message),
                        };

                        if !continues {
                            eprintln!("[LUA WARN] {}", message);
                            *current_message = None;
                        } else {
                            *current_message = Some(message);
                        }
                    }
                }
            }
            Ok(())
        }
    });

    for entry in EMBEDDED_DIR_MODULES.entries() {
        if let Some(module_file) = entry.as_file() {
            let name = module_file.path().file_stem().unwrap().to_string_lossy();
            let source = module_file.contents_utf8().unwrap();

            let module_name = format!("=module_{}", name);
            let loader = lua.create_function(move |l, ()| l.load(source).set_name(&module_name).eval::<mlua::Value>())?;

            lua.preload_module(&name, loader)?;
        }
    }

    let installs = lua
        .load(config_path)
        .set_name("=fab.lua")
        .eval::<HashMap<PathBuf, Either<UserDataRef<Artifact>, UserDataRef<Source>>>>()?;

    let installs = installs
        .into_iter()
        .map(|install| {
            (
                install.0,
                match install.1 {
                    Either::Left(v) => v.0.clone(),
                    Either::Right(v) => v.0.clone(),
                },
            )
        })
        .collect::<HashMap<_, _>>();

    let app_data = lua.remove_app_data::<FabricateAppData>().unwrap();

    Ok(Config {
        rules: app_data.rules,
        builds: app_data.builds,
        installs,
    })
}
