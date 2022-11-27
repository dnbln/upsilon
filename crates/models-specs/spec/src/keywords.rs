use crate::span::{Span, Spanned};

struct KwToken;

macro_rules! kw {
    ($name:ident) => {
        pub struct $name(Spanned<KwToken>);

        impl $name {
            pub fn span(&self) -> &Span {
                self.0.span()
            }
        }

        impl From<Spanned<KwToken>> for $name {
            fn from(spanned: Spanned<KwToken>) -> Self {
                Self(spanned)
            }
        }

        impl From<Span> for $name {
            fn from(span: Span) -> Self {
                Self(Spanned::new(KwToken, span))
            }
        }
    };
}

kw!(PackageKw);
kw!(NewtypeKw);
kw!(StructKw);
kw!(EnumKw);
kw!(GlueKw);
