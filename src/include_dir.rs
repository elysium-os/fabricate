use std::{
    fs::exists,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use mlua::{ExternalError, FromLua, Lua, Result, UserData, Value};

use crate::FabLuaContext;

#[derive(Clone, Debug)]
pub struct IncludeDirectory {
    pub path: PathBuf,
    pub filename: String,
}

impl UserData for IncludeDirectory {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("filename", |_, this| Ok(this.filename.clone()));
        fields.add_field_method_get("full_path", |_, this| Ok(this.path.clone()));
    }
}

impl FromLua for IncludeDirectory {
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        match value {
            Value::UserData(data) => Ok((*data.borrow::<IncludeDirectory>()?).clone()),
            _ => return Err(anyhow!("`{:?}` is not an IncludeDirectory", value).into_lua_err()),
        }
    }
}

impl IncludeDirectory {
    pub fn create(lua: &Lua, original_path: String) -> Result<IncludeDirectory> {
        let fab_context = lua.app_data_ref::<FabLuaContext>().unwrap();

        if !exists(&original_path)? {
            return Err(anyhow!("Include directory `{}` does not exist", original_path).into_lua_err());
        }

        let path = Path::new(&original_path).canonicalize()?.to_path_buf();
        if !path.is_dir() {
            return Err(anyhow!("Include directory `{}` is not a directory", original_path).into_lua_err());
        }

        let filename = match path.file_name() {
            None => return Err(anyhow!("Include directory `{}` has an invalid filename", original_path).into_lua_err()),
            Some(filename) => filename.to_owned().to_str().unwrap().to_string(),
        };

        if !path.starts_with(&fab_context.project_root) {
            return Err(anyhow!("Include directory `{}` is outside of the project root", original_path).into_lua_err());
        }

        Ok(IncludeDirectory {
            path: path.strip_prefix(&fab_context.project_root).unwrap().to_path_buf(),
            filename,
        })
    }
}
