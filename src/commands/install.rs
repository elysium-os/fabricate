use std::{
    fs::{copy, create_dir_all},
    path::{Component, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::{args::InstallArgs, cache::FabricateCache};

pub fn run(args: InstallArgs) -> Result<()> {
    let build_dir = PathBuf::from(args.build_dir)
        .canonicalize()
        .context("Failed to resolve build directory path")?;

    let cache = match FabricateCache::load(&build_dir.join("fabricate_cache.toml"))? {
        None => bail!("Cache is not initialized, make sure the build directory path is correct"),
        Some(cache) => cache,
    };

    for (dest, src) in cache.installs {
        let abs_src = build_dir.join(&src);
        let mut abs_dest = PathBuf::from(&cache.prefix).join(&dest);

        if let Some(dest_dir) = &args.dest_dir {
            let mut joined = PathBuf::from(dest_dir);

            for component in abs_dest.components() {
                match component {
                    Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
                    Component::ParentDir | Component::Normal(_) => joined.push(component),
                }
            }

            abs_dest = joined;
        }

        if !abs_src.exists() {
            bail!("Unable to install artifact `{}`, it does not exist", src.display());
        }

        if abs_src.is_dir() {
            bail!("Unable to install artifact `{}`, it is a directory", src.display());
        }

        let dest_dir = match abs_dest.parent() {
            None => bail!("Could not resolve directory path of destination"),
            Some(path) => path,
        };

        create_dir_all(dest_dir).with_context(|| {
            format!(
                "Unable to install artifact `{}`, could not create dest dir `{}`",
                src.display(),
                dest_dir.display()
            )
        })?;

        copy(&src, &abs_dest).with_context(|| {
            format!(
                "Unable to install artifact `{}` to `{}`, could not copy",
                src.to_string_lossy(),
                abs_dest.to_string_lossy()
            )
        })?;
    }

    Ok(())
}
