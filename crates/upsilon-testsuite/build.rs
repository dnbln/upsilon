fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=UPSILON_TESTSUITE_OFFLINE");
    if std::env::var("UPSILON_TESTSUITE_OFFLINE").is_ok() {
        println!("cargo:rustc-cfg=offline");
    }
}