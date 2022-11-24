use std::path::Path;
macro_rules! ws_path {
    ($($s:literal)/ * ) => {
        {
            use std::borrow::ToOwned;
            let mut p = $crate::ws::workspace_root().to_owned();
            $(
                p.push($s);
            )*
            p
        }
    }
}

macro_rules! ws_path_str {
    ($($s:literal)/ * ) => {
        $crate::ws::ws_path!($($s)/ *).to_str().unwrap().to_string()
    }
}

macro_rules! ws_path_join {
    (#[clone] $root:ident / $($s:literal)/ *) => {
        {
            let mut p = $root.clone();
            $(
                p.push($s);
            )*
            p
        }
    };
    ($root:ident / $($s:literal)/ *) => {
        {
            let mut p = $root;
            $(
                p.push($s);
            )*
            p
        }
    };
}

pub fn workspace_root() -> &'static Path {
    let xtask_dir: &Path = env!("CARGO_MANIFEST_DIR").as_ref();
    // parent of upsilon-xtask = crates,
    // parent of crates = workspace root
    xtask_dir.parent().unwrap().parent().unwrap()
}

pub(crate) use ws_path;
pub(crate) use ws_path_str;
pub(crate) use ws_path_join;