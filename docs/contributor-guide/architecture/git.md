---
sidebar_position: 4
---

# Interacting with `git` / `libgit2`

All the code that interacts with the `git` binary, or with `libgit2`, lies in
the following crates:

## `upsilon-vcs`

`upsilon-vcs` is the only crate in the main server process that interacts
with `libgit2`, all the other crates use `upsilon-vcs` to talk to `libgit2`.

As such, all the needed functionality of the webserver as far as git is
concerned happens through this crate, including set-up of repos.

It is also responsible for spawning `git` processes, like `git-http-backend`
and `git-daemon` to handle incoming requests, when necessary.

## `upsilon-git-hooks`

This is a binary crate, which is invoked by the various hooks in
`git-http-backend` and/or `git-daemon`, and is responsible for telling the
webserver what is actually happening in the repository. It also has the job of
rejecting specific actions, for example GitHub has protected branches, and won't
allow certain actions on them. This is done by passing the repo config from 
the webserver to the hook, through the `UPSILON_REPO_CONFIG` 
environment variable, which is then read by the hook (serialized JSON).

## `upsilon-git-protocol-accesshook`

Passed as `--access-hook` to `git-daemon`.