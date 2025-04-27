use std::path::PathBuf;

use mlua::{FromLua, Lua, Result, UserData, Value};

#[derive(Clone, Debug)]
pub struct Object {
    pub path: PathBuf,
}

impl UserData for Object {}

impl FromLua for Object {
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        match value {
            Value::UserData(data) => Ok((*data.borrow::<Object>()?).clone()),
            _ => panic!("value is not an object"),
        }
    }
}
