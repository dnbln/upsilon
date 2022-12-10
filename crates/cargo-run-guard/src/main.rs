fn main() {
    println!(
        r#"
Hello!

If you meant to run the dev version of upsilon,
you should use `cargo xtask run-dev` instead
(or `cargo x r` for short).

Or if you meant to pack for release, use
`cargo xtask pack-release`.
"#
    );

    std::process::exit(1);
}
