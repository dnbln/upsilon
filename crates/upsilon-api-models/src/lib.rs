use upsilon_procx::{dart_model_classes, DartModelClass};

dart_model_classes!();

#[derive(DartModelClass)]
pub struct F(String, Option<String>);

#[derive(DartModelClass)]
pub struct G {
    a: String,
    b: Option<String>,
    c: Vec<String>,
    d: F,
}
