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

macro_rules! model_module {
    ($name:ident) => {
        pub mod $name {
            include!(concat!(
                env!("OUT_DIR"),
                "/models.",
                stringify!($name),
                ".rs"
            ));
        }
    };
}

pub mod models {
    model_module!(namespace);
    model_module!(organizations);
    model_module!(repos);
    model_module!(users);
}

fn x() {
    // let id = models::namespace::NamespaceId { kind: Some(models::namespace::Kind::User) };
}
