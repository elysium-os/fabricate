use std::{fs::exists, path::PathBuf};

use mlua::{FromLua, Lua, Result, UserData, Value};

use crate::FabContext;

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
            _ => panic!("value is not a source"),
        }
    }
}

impl Source {
    pub fn create(lua: &Lua, original_path: PathBuf) -> Result<Source> {
        let fab_context = lua.app_data_ref::<FabContext>().unwrap();

        if !exists(&original_path)? {
            panic!("Source `{}` does not exist", original_path.to_str().unwrap());
        }

        let path = original_path.canonicalize()?.to_path_buf();
        if !path.is_file() {
            panic!("Source `{}` is not a file", original_path.to_str().unwrap());
        }

        let filename = match path.file_name() {
            None => panic!("Source `{}` has an invalid filename", original_path.to_str().unwrap()),
            Some(filename) => filename.to_owned().to_str().unwrap().to_string(),
        };

        if !path.starts_with(&fab_context.project_root) {
            panic!("Source `{}` is outside of the project root", original_path.to_str().unwrap());
        }

        Ok(Source {
            path: path.strip_prefix(&fab_context.project_root).unwrap().to_path_buf(),
            filename,
        })
    }
}
