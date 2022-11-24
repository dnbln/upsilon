pub type XtaskResult<T> = anyhow::Result<T>;

macro_rules! err {
    ($($t:tt),+) => {
        {
            eprintln!($($t,)+);
            std::process::exit(1)
        }
    };
}