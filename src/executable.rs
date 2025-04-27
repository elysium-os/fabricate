use std::path::PathBuf;

use mlua::{FromLua, Lua, Result, UserData, Value};
use regex::Regex;
use which::{which, which_re};

#[derive(Clone, Debug)]
pub struct Executable {
    pub path: PathBuf,
    pub filename: String,
}

impl UserData for Executable {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("filename", |_, this| Ok(this.filename()))
    }
}

impl FromLua for Executable {
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        match value {
            Value::UserData(data) => Ok((*data.borrow::<Executable>()?).clone()),
            _ => panic!("value is not an executable"),
        }
    }
}

impl Executable {
    pub fn find(_: &Lua, search: String) -> Result<Option<Executable>> {
        let bin_path = match &search {
            search if search.chars().all(char::is_alphanumeric) => match which(search) {
                Ok(path) => Some(path),
                Err(_) => None,
            },
            search => match &mut which_re(Regex::new(search).unwrap_or_else(|err| panic!("Invalid search_bin regex `{}`: {}", search, err))) {
                Ok(paths) => paths.next(),
                Err(_) => None,
            },
        };

        Ok(match bin_path {
            Some(path) => {
                let filename = match path.file_name() {
                    None => panic!("Executable `{}` has an invalid filename", path.to_str().unwrap()),
                    Some(filename) => filename.to_owned().to_str().unwrap().to_string(),
                };
                Some(Executable { path, filename })
            }
            None => None,
        })
    }

    pub fn filename(&self) -> String {
        self.filename.clone()
    }
}
