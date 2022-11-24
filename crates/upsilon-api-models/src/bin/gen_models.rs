use std::fmt::Write;
use std::path::PathBuf;

fn main() {
    let target_file = PathBuf::from(
        std::env::args()
            .skip(1)
            .next()
            .expect("Missing path to file"),
    );

    let mut file_contents = String::from(
        "\
// GENERATED CODE - DO NOT MODIFY BY HAND




",
    );

    for class in upsilon_api_models::__DART_MODEL_CLASSES.iter() {
        let (_name, dart_class_decl) = class();

        write!(&mut file_contents, "{}", dart_class_decl).unwrap();
    }

    std::fs::write(target_file, file_contents).expect("Cannot write");
}
