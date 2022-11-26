fn main() {
    lalrpop::Configuration::new()
        .always_use_colors()
        .generate_in_source_tree()
        .process_file("src/parser.lalrpop")
        .unwrap();
}
