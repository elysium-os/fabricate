use std::{fs::exists, path::PathBuf};

use mlua::{FromLua, Lua, Result, UserData, Value};

use crate::FabContext;

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
            _ => panic!("value is not an include directory"),
        }
    }
}

impl IncludeDirectory {
    pub fn create(lua: &Lua, original_path: PathBuf) -> Result<IncludeDirectory> {
        let fab_context = lua.app_data_ref::<FabContext>().unwrap();

        if !exists(&original_path)? {
            panic!("Include directory `{}` does not exist", original_path.to_str().unwrap());
        }

        let path = original_path.canonicalize()?.to_path_buf();
        if !path.is_dir() {
            panic!("Include directory `{}` is not a directory", original_path.to_str().unwrap());
        }

        let filename = match path.file_name() {
            None => panic!("Include directory `{}` has an invalid filename", original_path.to_str().unwrap()),
            Some(filename) => filename.to_owned().to_str().unwrap().to_string(),
        };

        if !path.starts_with(&fab_context.project_root) {
            panic!("Include directory `{}` is outside of the project root", original_path.to_str().unwrap());
        }

        Ok(IncludeDirectory {
            path: path.strip_prefix(&fab_context.project_root).unwrap().to_path_buf(),
            filename,
        })
    }
}
