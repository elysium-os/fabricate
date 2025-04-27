use core::panic;
use std::{
    fs::{exists, remove_dir_all},
    path::PathBuf,
};

use git2::{build::RepoBuilder, FetchOptions};
use glob::glob;
use mlua::{FromLua, Lua, Result, UserData, Value};

use crate::FabContext;

#[derive(Clone, Debug)]
pub struct Dependency {
    name: String,
}

impl UserData for Dependency {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.name.clone()));
        fields.add_field_method_get("path", |lua: &Lua, this| {
            let fab_context = lua.app_data_ref::<FabContext>().unwrap();
            Ok(fab_context.path_dependencies().join(&this.name))
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("glob", |lua: &Lua, this, pattern: String| {
            let fab_context = lua.app_data_ref::<FabContext>().unwrap();

            let mut paths: Vec<PathBuf> = Vec::new();
            for entry in glob(fab_context.path_dependencies().join(&this.name).join(&pattern).to_str().unwrap())
                .unwrap_or_else(|err| panic!("Glob pattern `{}` failed: {}", pattern, err))
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
            _ => panic!("value is not a dependency"),
        }
    }
}

impl Dependency {
    pub fn create(lua: &Lua, (name, url, revision): (String, String, String)) -> Result<Dependency> {
        let mut fab_context = lua.app_data_mut::<FabContext>().unwrap();

        if !name.chars().all(|ch| ch.is_alphanumeric() || ch == '-' || ch == '_') {
            panic!("Dependency name `{}` is not alphanumeric, not '-', and not '_'", name);
        }

        if fab_context.dependency_cache.contains(&name) {
            panic!("Dependency `{}` defined more than once", name);
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
            .unwrap_or_else(|err| panic!("Failed to clone dependency `{}`: {}", name, err));

        let obj = repo
            .revparse_single(revision.as_str())
            .unwrap_or_else(|err| panic!("Unable to find revision `{}`: {}", revision, err));

        repo.checkout_tree(&obj, None)
            .unwrap_or_else(|err| panic!("Failed to checkout revision `{}`: {}", revision, err));

        Ok(Dependency { name })
    }
}
