use crate::span::{Span, Spanned};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct PunctToken;

macro_rules! punct {
    ($name:ident) => {
        #[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
        pub struct $name(Spanned<PunctToken>);

        impl $name {
            pub fn span(&self) -> &Span {
                self.0.span()
            }
        }

        impl From<Spanned<PunctToken>> for $name {
            fn from(spanned: Spanned<PunctToken>) -> Self {
                Self(spanned)
            }
        }

        impl From<Span> for $name {
            fn from(span: Span) -> Self {
                Self(Spanned::new(PunctToken, span))
            }
        }
    };
}

punct!(ColonPunctToken);
punct!(SemicolonPunctToken);
punct!(CommaPunctToken);
punct!(DotPunctToken);
punct!(OpenParenPunctToken);
punct!(CloseParenPunctToken);
punct!(OpenBracePunctToken);
punct!(CloseBracePunctToken);
punct!(OpenBracketPunctToken);
punct!(CloseBracketPunctToken);
punct!(OpenAngleBracketPunctToken);
punct!(CloseAngleBracketPunctToken);
punct!(QMarkPunctToken);
punct!(EqPunctToken);
punct!(HashPunctToken);
