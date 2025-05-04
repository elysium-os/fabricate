use std::{
    fs::{exists, remove_dir_all},
    path::PathBuf,
};

use anyhow::{anyhow, Context};
use git2::{build::RepoBuilder, FetchOptions};
use glob::glob;
use mlua::{ExternalError, ExternalResult, FromLua, Lua, Result, UserData, Value};

use crate::FabLuaContext;

#[derive(Clone, Debug)]
pub struct Dependency {
    name: String,
}

impl UserData for Dependency {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.name.clone()));
        fields.add_field_method_get("path", |lua: &Lua, this| {
            let fab_context = lua.app_data_ref::<FabLuaContext>().unwrap();
            Ok(fab_context.path_dependencies().join(&this.name))
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("glob", |lua: &Lua, this, pattern: String| {
            let fab_context = lua.app_data_ref::<FabLuaContext>().unwrap();

            let mut paths: Vec<PathBuf> = Vec::new();
            for entry in glob(fab_context.path_dependencies().join(&this.name).join(&pattern).to_str().unwrap())
                .with_context(|| format!("Glob pattern `{}` failed", pattern))
                .into_lua_err()?
            {
                match entry {
                    Ok(path) => paths.push(path),
                    Err(_) => continue,
                }
            }
            Ok(paths)
        })
    }
}

impl FromLua for Dependency {
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        match value {
            Value::UserData(data) => Ok((*data.borrow::<Dependency>()?).clone()),
            _ => return Err(anyhow!("`{:?}` is not a Dependency", value).into_lua_err()),
        }
    }
}

impl Dependency {
    pub fn create(lua: &Lua, (name, url, revision): (String, String, String)) -> Result<Dependency> {
        let mut fab_context = lua.app_data_mut::<FabLuaContext>().unwrap();

        if !name.chars().all(|ch| ch.is_alphanumeric() || ch == '-' || ch == '_') {
            return Err(anyhow!("Dependency name `{}` is not alphanumeric, not '-', and not '_'", name).into_lua_err());
        }

        if fab_context.dependency_cache.contains(&name) {
            return Err(anyhow!("Dependency `{}` defined more than once", name).into_lua_err());
        }
        fab_context.dependency_cache.push(name.clone());

        let dependency_path = fab_context.path_dependencies().join(&name);
        if exists(&dependency_path)? {
            remove_dir_all(&dependency_path)?;
        }

        let mut fetch_options = FetchOptions::new();
        fetch_options.depth(0);

        let repo = RepoBuilder::new()
            .fetch_options(fetch_options)
            .clone(url.as_str(), &dependency_path)
            .with_context(|| format!("Failed to clone dependency `{}`", name))
            .into_lua_err()?;

        let obj = repo
            .revparse_single(revision.as_str())
            .with_context(|| format!("Unable to find revision `{}`", revision))
            .into_lua_err()?;

        repo.checkout_tree(&obj, None)
            .with_context(|| format!("Failed to checkout revision `{}`", revision))
            .into_lua_err()?;

        Ok(Dependency { name })
    }
}
