# Testing

All integration tests go in `crates/upsilon-testsuite`, with
`crates/upsilon-test-support` providing some utilities for them.

## Running the tests

To run the tests, you can use the `test` xtask:

```bash
cargo xtask test
# Or for short
cargo xt
```

## Writing tests

Integration tests are annotated with `#[upsilon_test]`, which handles most of
the setup, and provides a `TestCx` to the test, which is used to interact with
the webserver.

By default, the test server doesn't spawn a `git daemon`, but can be configured
to do so by annotating the `TestCx` parameter
with `#[cfg_setup(upsilon_basic_config_with_git_daemon)]`.

At the very end, `#[upsilon_test]` will also make sure to clean up the web
server and terminate any subprocesses it has spawned.

All tests return a `TestResult<()>`, which is just
a `Result<(), anyhow::Error>`. For the cleanup to work, tests should
return `Err` if they fail, rather than `panic!`.

## Cleanup: internals

The webserver is spawned in a separate process, under a
`upsilon-gracefully-shutdown-host` process that acts as a middle-man, and
listens via `ctrlc` to signals, or to the creation of a temporary file.

### Windows

The `upsilon-gracefully-shutdown-host` process is created with
the `CREATE_NEW_PROCESS_GROUP` flag. When it receives an event
(via `ctrlc`), or the temporary file is created, it generates a
`CTRL_BREAK_EVENT` for itself, which is then propagated to all the child
processes, thus shutting everything down.

### Linux

The `upsilon-gracefully-shutdown-host` process just waits for `SIGTERM`,
`SIGINT` or similar signals via `ctrlc`, then walks the `procfs` to find all the
descendants of the process, and sends `SIGTERM` to each of them.

### Other platforms

`procfs` is only available on Linux, so separate implementations are needed for
other platforms. The `upsilon-gracefully-shutdown-host` binary just fails to
compile on other platforms right now, making it impossible to run tests on them.