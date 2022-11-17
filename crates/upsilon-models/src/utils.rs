macro_rules! str_newtype {
    ($name:ident) => {
        #[derive(
            serde::Serialize, serde::Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Hash,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl<'a> From<&'a str> for $name {
            fn from(s: &'a str) -> Self {
                Self(s.to_owned())
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

macro_rules! qerror {
    ($vis:vis $name:ident, $($variant:ident : $error:literal),* $(,)?) => {
        #[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $vis enum $name {
            $(
                #[error($error)]
                $variant,
            )*
        }
    };
}

pub(crate) use qerror;
pub(crate) use str_newtype;
