use std::path::PathBuf;

fn main() {
    let s = r##"
package users {
    #[uuid]
    newtype<UUID> struct UserID;
    #[str]
    newtype<str> struct Username;
    #[str]
    newtype<str> struct UserDisplayName;

    struct User {
        id: UserID,
        username: Username,
        name?: UserDisplayName,
    }

    struct UserSelf {
        id: UserID,
        username: Username,
        name?: UserDisplayName,
    }
}
    "##;

    let (parsed, diagnostics) =
        spec::parse(Some(PathBuf::from("aaa.modelspec")), s).expect("parse");
    let _ = spec::compile(parsed, &diagnostics);

    diagnostics.emit();
}
