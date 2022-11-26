macro_rules! models {
    ($($name:ident),* $(,)?) => {
        const MODELS: &[&str] = &[
            $(
                concat!("../../api-models/protobuf/", stringify!($name), ".proto"),
            )*
        ];
    };
}

models! {
    namespace,
    organizations,
    repos,
    users,
}

fn main() {
    println!("cargo:rerun-if-changed=../../api-models/protobuf");
    println!("cargo:rerun-if-changed=build.rs");

    prost_build::Config::new()
        .format(true)
        .compile_protos(MODELS, &["../../api-models/protobuf"])
        .expect("prost_build");
}
