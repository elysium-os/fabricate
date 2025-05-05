use std::ffi::OsString;

use anyhow::anyhow;
use json::object;
use mlua::{ExternalError, Function, Lua, Result, Table, UserData};
use ninja_writer::{escape, escape_path, BuildVariables, RuleRef, RuleVariables, Variables};

use crate::{executable::Executable, include_dir::IncludeDirectory, object::Object, source::Source, FabLuaContext};

#[derive(Clone, Debug)]
pub struct Compiler {
    name: String,
    format_include_dir: Option<Function>,
    compile_command_format: Option<String>,
    build_rule: RuleRef,
}

impl UserData for Compiler {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.name.clone()));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "build",
            |lua: &Lua, this, (sources, mut args, includes): (Vec<Source>, Vec<String>, Option<Vec<IncludeDirectory>>)| {
                let mut fab_context = lua.app_data_mut::<FabLuaContext>().unwrap();

                args = args.into_iter().map(|arg| escape(arg.as_str()).to_string()).collect();
                if let Some(includes) = &includes {
                    if let Some(format_include_dir) = &this.format_include_dir {
                        for include in includes {
                            args.insert(
                                0,
                                escape_path(format_include_dir.call::<String>(fab_context.project_root.join(include.path.clone()))?.as_str()).to_string(),
                            );
                        }
                    } else {
                        return Err(anyhow!("Compiler `{}` does not support includes", this.name).into_lua_err());
                    }
                }

                let mut objects: Vec<Object> = Vec::new();
                for source in &sources {
                    let mut extension = match source.path.extension() {
                        None => OsString::new(),
                        Some(ext) => ext.to_owned(),
                    };

                    let mut source_path_components: Vec<String> = Vec::new();
                    for component in source.path.components() {
                        source_path_components.push(escape_path(component.as_os_str().to_str().unwrap()).replace("_", "__"));
                    }
                    let source_path_str = source_path_components.join("_");

                    extension.push(".o");
                    let object_path = fab_context.build_cache.join("objects").join(&source_path_str).with_extension(&extension);

                    extension.push(".d");
                    let depfile_path = fab_context.build_cache.join("depfiles").join(&source_path_str).with_extension(&extension);

                    let mut build = this.build_rule.build([&object_path]).with([&fab_context.project_root.join(&source.path)]);
                    build = build.variable("depfile", depfile_path);
                    build.variable("arguments", args.join(" "));

                    objects.push(Object { path: object_path.clone() });

                    // Compile Commands
                    let mut compile_command = Vec::new();
                    if let Some(fmt) = &this.compile_command_format {
                        for arg in fmt.split_whitespace() {
                            compile_command.push(match arg {
                                "@FLAGS@" => args.join(" "),
                                "@IN@" => source.path.to_str().unwrap().to_string(),
                                "@OUT@" => object_path.to_str().unwrap().to_string(),
                                _ => arg.to_string(),
                            })
                        }
                    }

                    let compile_command = object! {
                        directory: fab_context.build_cache.to_str().unwrap(),
                        arguments: compile_command,
                        file: source.path.to_str().unwrap(),
                        output: object_path.to_str().unwrap()
                    };

                    fab_context.compile_commands.push(compile_command);
                }

                Ok(objects)
            },
        );
    }
}

impl Compiler {
    pub fn create(lua: &Lua, table: Table) -> Result<Compiler> {
        let mut fab_context = lua.app_data_mut::<FabLuaContext>().unwrap();

        let name: String = escape(table.get::<String>("name")?.as_str()).to_string();
        let executable: Executable = table.get("executable")?;
        let command: String = escape(table.get::<String>("command")?.as_str()).to_string();
        let mut description: Option<String> = None;
        if table.contains_key("description")? {
            description = Some(escape(table.get::<String>("description")?.as_str()).to_string());
        }
        let mut format_include_dir: Option<Function> = None;
        if table.contains_key("format_include_dir")? {
            format_include_dir = Some(table.get("format_include_dir")?);
        }
        let mut compile_command_format: Option<String> = None;
        if table.contains_key("compile_command_format")? {
            compile_command_format = Some(table.get("compile_command_format")?);
        }

        let rule_name = name.clone() + "_build";
        if fab_context.rule_cache.contains(&rule_name) {
            return Err(anyhow!("Rule `{}` defined more than once", rule_name).into_lua_err());
        }
        fab_context.rule_cache.push(rule_name.clone());

        let mut build_command: Vec<&str> = Vec::new();
        for arg in command.split_whitespace() {
            build_command.push(match arg {
                "@EXEC@" => executable.path.to_str().unwrap(),
                "@DEPFILE@" => "$depfile",
                "@FLAGS@" => "$arguments",
                "@IN@" => "$in",
                "@OUT@" => "$out",
                arg if arg.starts_with("@") && arg.ends_with("@") => {
                    return Err(anyhow!("Unknown embed `{}` in compiler `{}`", arg, name).into_lua_err());
                }
                _ => arg,
            });
        }

        let mut build_rule = fab_context.ninja.rule(&rule_name, build_command.join(" "));
        if let Some(description) = description {
            build_rule = build_rule.description(description.replace("@OUT@", "$out").replace("@IN@", "$in"));
        }

        build_rule = match executable.filename().as_str() {
            exe if exe.ends_with("clang") => build_rule.deps_gcc(),
            exe if exe.ends_with("gcc") => build_rule.deps_gcc(),
            "msvc" => build_rule.deps_msvc(),
            _ => build_rule,
        };

        if let Some(fmt) = compile_command_format {
            compile_command_format = Some(fmt.replace("@EXEC@", executable.path.to_str().unwrap()));
        }

        Ok(Compiler {
            name,
            format_include_dir,
            compile_command_format,
            build_rule,
        })
    }
}
