use std::{
    fs::exists,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use mlua::{ExternalError, FromLua, Lua, Result, UserData, Value};

use crate::FabLuaContext;

#[derive(Clone, Debug)]
pub struct Source {
    pub path: PathBuf,
    pub filename: String,
}

impl UserData for Source {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("filename", |_, this| Ok(this.filename.clone()));
        fields.add_field_method_get("full_path", |_, this| Ok(this.path.clone()));
    }
}

impl FromLua for Source {
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        match value {
            Value::UserData(data) => Ok((*data.borrow::<Source>()?).clone()),
            _ => return Err(anyhow!("`{:?}` is not a Source", value).into_lua_err()),
        }
    }
}

impl Source {
    pub fn create(lua: &Lua, original_path: String) -> Result<Source> {
        let fab_context = lua.app_data_ref::<FabLuaContext>().unwrap();

        if !exists(&original_path)? {
            return Err(anyhow!("Source `{}` does not exist", original_path).into_lua_err());
        }

        let path = Path::new(&original_path).canonicalize()?.to_path_buf();
        if !path.is_file() {
            return Err(anyhow!("Source `{}` is not a file", original_path).into_lua_err());
        }

        let filename = match path.file_name() {
            None => return Err(anyhow!("Source `{}` has an invalid filename", original_path).into_lua_err()),
            Some(filename) => filename.to_str().unwrap().to_string(),
        };

        if !path.starts_with(&fab_context.project_root) {
            return Err(anyhow!("Source `{}` is outside of the project root", original_path).into_lua_err());
        }

        Ok(Source {
            path: path.strip_prefix(&fab_context.project_root).unwrap().to_path_buf(),
            filename,
        })
    }
}
