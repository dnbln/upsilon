mod compiler;

pub use compiler::SerdeCompiler;

pub type Config = spec::Config<SerdeCompiler>;
