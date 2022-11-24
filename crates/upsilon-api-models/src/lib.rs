use upsilon_procx::{dart_model_classes, DartModelClass};


dart_model_classes!();

#[derive(DartModelClass)]
pub struct F(String, Option<String>, Vec<String>, (String, i32));