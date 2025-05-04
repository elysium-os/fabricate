use std::path::PathBuf;

use anyhow::anyhow;
use mlua::{ExternalError, FromLua, Lua, Result, UserData, Value};

#[derive(Clone, Debug)]
pub struct Object {
    pub path: PathBuf,
}

impl UserData for Object {}

impl FromLua for Object {
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        match value {
            Value::UserData(data) => Ok((*data.borrow::<Object>()?).clone()),
            _ => return Err(anyhow!("`{:?} is not an Object`", value).into_lua_err()),
        }
    }
}
