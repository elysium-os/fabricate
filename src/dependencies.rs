use std::{
    collections::HashMap,
    fs::read_to_string,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use git2::{Repository, build::CheckoutBuilder};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct GitRepository {
    pub url: String,
    pub revision: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Dependency {
    Git {
        #[serde(flatten)]
        repository: GitRepository,
    },
}

pub fn read_dependencies_file(path: &Path) -> Result<HashMap<String, Dependency>> {
    let data = read_to_string(path)?;
    let deps = toml::from_str::<HashMap<String, Dependency>>(&data)?;
    Ok(deps)
}

pub fn fetch_dependencies(deps_dir: &Path, fetch: bool, dependencies: &HashMap<String, Dependency>) -> Result<HashMap<String, PathBuf>> {
    let mut resolved_dependencies = HashMap::new();
    for (name, dep) in dependencies {
        let path = deps_dir.join(&name);
        match dep {
            Dependency::Git { repository } => {
                if !path.exists() {
                    if !fetch {
                        continue;
                    }
                    eprintln!("Cloning git repository `{}`...", name);
                    clone_repo(&path, &repository).with_context(|| format!("Failed to fetch git dependency `{}`", name))?;
                } else if !is_up_to_date(&path, &repository).with_context(|| format!("Failed to check git dependency `{}`", name))? {
                    if !fetch {
                        continue;
                    }
                    eprintln!("Updating git repository `{}`...", name);
                    update_repo(&path, &repository).with_context(|| format!("Failed to update git dependency `{}`", name))?;
                }
            }
        }
        resolved_dependencies.insert(name.clone(), path);
    }

    Ok(resolved_dependencies)
}

fn is_up_to_date(repo_dir: &Path, repo: &GitRepository) -> Result<bool> {
    let git_repo = Repository::open(repo_dir)?;

    let url_matches = git_repo.find_remote("origin")?.url().is_ok_and(|url| url == repo.url);

    if !url_matches {
        return Ok(false);
    }

    let revision_matches = git_repo.revparse_single(&repo.revision).ok().map(|obj| obj.id()) == git_repo.head()?.target();

    Ok(revision_matches)
}

fn clone_repo(repo_dir: &Path, repo: &GitRepository) -> Result<()> {
    let git_repo = Repository::clone(&repo.url, repo_dir).with_context(|| format!("Failed to clone `{}`", repo.url))?;

    checkout_revision(&git_repo, &repo.revision)
}

fn update_repo(repo_dir: &Path, repo: &GitRepository) -> Result<()> {
    let git_repo = Repository::open(repo_dir).with_context(|| format!("Failed to open repository at `{}`", repo_dir.display()))?;

    git_repo.remote_set_url("origin", &repo.url).context("Failed to update remote url")?;

    git_repo
        .find_remote("origin")?
        .fetch(&[] as &[&str], None, None)
        .context("Failed to fetch")?;

    checkout_revision(&git_repo, &repo.revision)
}

fn checkout_revision(git_repo: &Repository, revision: &str) -> Result<()> {
    let obj = git_repo
        .revparse_single(revision)
        .with_context(|| format!("Failed to resolve revision `{revision}`"))?;

    git_repo
        .checkout_tree(&obj, Some(&mut CheckoutBuilder::new().force()))
        .context("Failed to checkout revision")?;

    git_repo.set_head_detached(obj.id()).context("Failed to set HEAD")
}
