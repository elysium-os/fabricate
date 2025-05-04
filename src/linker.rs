use anyhow::anyhow;
use mlua::{ExternalError, Lua, Result, Table, UserData};
use ninja_writer::{escape, BuildVariables, RuleRef, RuleVariables, Variables};

use crate::{executable::Executable, object::Object, FabLuaContext};

#[derive(Clone, Debug)]
pub struct Linker {
    name: String,
    rule: RuleRef,
}

impl UserData for Linker {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.name.clone()));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("link", |_: &Lua, this, (objects, args, output_filename): (Vec<Object>, Vec<String>, String)| {
            this.rule
                .build([output_filename])
                .with(objects.into_iter().map(|obj| obj.path))
                .variable("arguments", args.join(" "));

            Ok(())
        });
    }
}

impl Linker {
    pub fn create(lua: &Lua, table: Table) -> Result<Linker> {
        let mut fab_context = lua.app_data_mut::<FabLuaContext>().unwrap();

        let name: String = escape(table.get::<String>("name")?.as_str()).to_string();
        let executable: Executable = table.get("executable")?;
        let command: String = escape(table.get::<String>("command")?.as_str()).to_string();
        let mut description: Option<String> = None;
        if table.contains_key("description")? {
            description = Some(escape(table.get::<String>("description")?.as_str()).to_string());
        }

        let rule_name = name.clone() + "_link";
        if fab_context.rule_cache.contains(&name) {
            return Err(anyhow!("Rule `{}` defined more than once", rule_name).into_lua_err());
        }
        fab_context.rule_cache.push(name.clone());

        let mut link_command: Vec<&str> = Vec::new();
        for arg in command.split_whitespace() {
            link_command.push(match arg {
                "@EXEC@" => executable.path.to_str().unwrap(),
                "@FLAGS@" => "$arguments",
                "@IN@" => "$in",
                "@OUT@" => "$out",
                arg if arg.starts_with("@") && arg.ends_with("@") => {
                    return Err(anyhow!("Unknown embed `{}` in compiler linker `{}`", arg, name).into_lua_err());
                }
                _ => arg,
            });
        }

        let mut rule = fab_context.ninja.rule(&rule_name, link_command.join(" "));
        if let Some(description) = description {
            rule = rule.description(&description.replace("@OUT@", "$out").replace("@IN@", "$in"))
        }

        Ok(Linker { name, rule })
    }
}
