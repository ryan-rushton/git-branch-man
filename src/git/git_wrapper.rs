use std::process::Command;

use regex::Regex;
use tracing::{error, info};

use crate::error::Error;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GitRemoteBranch {
  pub name: String,
}

impl GitRemoteBranch {
  pub fn new(name: String) -> Self {
    GitRemoteBranch { name }
  }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GitBranch {
  pub name: String,
  pub is_head: bool,
  pub upstream: Option<GitRemoteBranch>,
}

impl GitBranch {
  pub fn new(name: String) -> Self {
    GitBranch { name, is_head: false, upstream: None }
  }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GitStash {
  pub index: usize,
  pub message: String,
  pub stash_id: String,
}

impl GitStash {
  pub fn new(index: usize, message: String, stash_id: String) -> Self {
    GitStash { index, message, stash_id }
  }
}

pub async fn git_local_branches() -> Result<Vec<GitBranch>, Error> {
  let res = run_git_command(&["branch", "--list", "-vv"]).await?;

  let branches: Vec<GitBranch> = res
    .lines()
    .map(|line| {
      let trimmed = line.trim();
      // A regex to capture the following git list outputs
      // * git-cli-repo 911ec26 [origin/git-cli-repo] Linting
      //   main         8fb5d9b [origin/main] Fix build
      //   stash-list   6442450 [origin/stash-list: gone] Formatting
      //   test         dbcf785 Updates
      let re = Regex::new(
        r"((?<head>\*)\s+)?(?<name>\S+)\s+(?<sha>[A-Fa-f0-9]+)\s+(\[(?<upstream>[^:|^\]]+)(?<gone>[:\sgone]+)?)?",
      )
      .unwrap();
      let Some(captures) = re.captures(trimmed) else {
        error!("Failed to capture git branch information for: {}", trimmed);
        return GitBranch::new(String::from(trimmed));
      };
      let is_head = captures.name("head").is_some();
      let name = String::from(captures.name("name").unwrap().as_str());
      let upstream = captures.name("upstream");
      GitBranch {
        name,
        is_head,
        upstream: upstream.map(|upstream_name| GitRemoteBranch::new(String::from(upstream_name.as_str()))),
      }
    })
    .collect();

  Ok(branches)
}

pub async fn git_stashes() -> Result<Vec<GitStash>, Error> {
  let res = run_git_command(&["branch", "--list"]).await?;

  let stashes: Vec<GitStash> = res
    .lines()
    .enumerate()
    .map(|(index, line)| GitStash::new(index, String::from(line.trim()), String::new()))
    .collect();

  Ok(stashes)
}

pub async fn git_checkout_branch_from_name(branch_name: &str) -> Result<(), Error> {
  run_git_command(&["checkout", branch_name]).await?;
  Ok(())
}

pub async fn git_checkout_branch(branch: &GitBranch) -> Result<(), Error> {
  git_checkout_branch_from_name(&branch.name).await
}

pub async fn git_validate_branch_name(name: &str) -> Result<bool, Error> {
  let res = run_git_command(&["check-ref-format", "--branch", name]).await;
  Ok(res.is_ok())
}

pub async fn git_create_branch(to_create: &GitBranch) -> Result<(), Error> {
  run_git_command(&["checkout", "-b", &to_create.name]).await?;
  Ok(())
}

pub async fn git_delete_branch(to_delete: &GitBranch) -> Result<(), Error> {
  run_git_command(&["branch", "-D", &to_delete.name]).await?;
  Ok(())
}

async fn run_git_command(args: &[&str]) -> Result<String, Error> {
  let args_log_command = args.join(" ");
  info!("Running `git {}`", args_log_command);
  let res = Command::new("git").args(args).output();
  if res.is_err() {
    let err = res.err().unwrap();
    error!("Failed to run `git {}`, error: {}", args_log_command, err);
    return Err(Error::Git(format!("{}", err)));
  }

  let output = res.unwrap();
  let err = String::from_utf8(output.stderr)?;
  if !output.status.success() && !err.is_empty() {
    error!("Failed to run `git {}`, error: {}", args_log_command, err);
    return Err(Error::Git(err));
  }
  let content = String::from_utf8(output.stdout)?;
  info!("Received git cli reply:\n{}", content.trim());
  Ok(content)
}
