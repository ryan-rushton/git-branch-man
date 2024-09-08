# git-branch-manager

![CI](https://github.com/ryan-rushton/git-branch-manager/actions/workflows/ci.yml/badge.svg)

Git branch and stash manager

## Dev Setup OSX

1. Install `brew install direnv`
2. Make sure to setup direnv for your profile `eval "$(direnv hook zsh)"`
3. Run `direnv allow` in this root dir

## Logs

Logs can be found at `~/Library/Application Support/com.rrushton.git-branch-manager/git-branch-manager.log` for prod
builds and in `.data/` for dev builds.

## TODO List

- Multiple git implementations so we can have a faster non-cli version for repo's that can use it. Originally I had two,
  but being new to rust dealing with an async injected repository was a yak shave I didn't need at the time. So, I
  decided to strip it back to just a CLI impl until a point where I was happy enough with that being async that I try
  and replace it with an injected git2 or gitoxide version.