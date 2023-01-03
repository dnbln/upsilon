# API

In terms of the API itself, there are multiple components.

## GraphQL API

The GraphQL API contains the main API for the webserver. It is implemented in
the `upsilon-api` crate.

## Git HTTP APIs

For the webserver to work with git clients over `http://`, it should invoke the
`git-http-backend` CGI backend, which is provided by `git` itself, on a few
certain paths.

The mechanism for passing the request to `git-http-backend` is implemented in
the `upsilon-web` and `upsilon-vcs` crates.
