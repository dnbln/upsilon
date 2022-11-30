/*
 *        Copyright (c) 2022 Dinu Blanovschi
 *
 *    Licensed under the Apache License, Version 2.0 (the "License");
 *    you may not use this file except in compliance with the License.
 *    You may obtain a copy of the License at
 *
 *        https://www.apache.org/licenses/LICENSE-2.0
 *
 *    Unless required by applicable law or agreed to in writing, software
 *    distributed under the License is distributed on an "AS IS" BASIS,
 *    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *    See the License for the specific language governing permissions and
 *    limitations under the License.
 */

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
