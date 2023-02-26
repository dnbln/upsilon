/*
 *        Copyright (c) 2022-2023 Dinu Blanovschi
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

#![feature(const_trait_impl)]
#![feature(const_mut_refs)]

mod also {
    //! Kotlin-like `also` expressions.

    /// Trait to implement `also` on all types
    pub trait Also: Sized {
        /// Kotlin-like `also` extension function
        #[inline(always)]
        fn also<F, T>(mut self, f: F) -> Self
        where
            F: FnOnce(&mut Self) -> T,
        {
            let _ = f(&mut self);
            self
        }
    }

    impl<T> Also for T {}
}

pub mod clone_to {
    //! Holds the [`CloneTo`] trait

    /// Trait to define `clone_to*` on all types
    ///
    /// # Examples
    ///
    /// ```
    /// # use upsilon_stdx::CloneTo;
    /// let mut a = 0;
    /// assert_eq!(&1, 1_i32.clone_to_ref(&mut a));
    /// assert_eq!(1, a);
    /// ```
    ///
    /// ```
    /// # use upsilon_stdx::CloneTo;
    /// let mut a = 0;
    /// assert_eq!(&mut 1, 1_i32.clone_to_ref_mut(&mut a));
    /// assert_eq!(1, a);
    /// ```
    ///
    /// ```
    /// # use upsilon_stdx::CloneTo;
    /// let mut a = 0;
    /// assert_eq!(1, 1_i32.clone_to(&mut a));
    /// assert_eq!(1, a);
    /// ```
    pub trait CloneTo: Clone {
        /// Clones other from self and returns self; reference variant.
        #[inline(always)]
        fn clone_to_ref(&self, other: &mut Self) -> &Self {
            other.clone_from(self);
            self
        }

        /// Clones other from self and returns self; mutable reference variant.
        #[inline(always)]
        fn clone_to_ref_mut(&mut self, other: &mut Self) -> &mut Self {
            other.clone_from(self);
            self
        }

        /// Clones other from self and returns self; owned variant.
        #[inline(always)]
        fn clone_to(self, other: &mut Self) -> Self
        where
            Self: Sized,
        {
            other.clone_from(&self);
            self
        }
    }

    impl<T: Clone + ?Sized> CloneTo for T {}
}

mod copy_to {
    //! Holds the [`CopyTo`] trait.

    /// A simple trait that with `copy_to`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use upsilon_stdx::CopyTo;
    /// #
    /// let mut i = 1;
    /// assert_eq!(2, 2.copy_to(&mut i));
    /// assert_eq!(i, 2);
    /// ```
    pub trait CopyTo: Copy {
        /// Copies self to other and returns self
        #[inline(always)]
        fn copy_to(self, other: &mut Self) -> Self {
            *other = self;
            self
        }
    }

    impl<T: Copy> CopyTo for T {}
}

mod with {
    //! Kotlin-like `let` expressions.
    //! But due to rust's `let` keyword, it's not possible
    //! to use it so they are renamed to `with`

    /// Trait to implement `with` on all types.
    ///
    /// # Examples
    ///
    /// ```
    /// # use upsilon_stdx::With;
    /// assert_eq!(2, 1.with(|it| it + 1));
    /// ```
    #[const_trait]
    pub trait With: Sized {
        /// Convert this object with a closure
        #[inline(always)]
        fn with<F, T>(self, f: F) -> T
        where
            F: ~const FnOnce(Self) -> T,
        {
            f(self)
        }
    }

    /// Trait to implement `with_ref` on all types.
    ///
    /// # Examples
    ///
    /// ```
    /// # use upsilon_stdx::WithRef;
    /// assert_eq!(String::from("aaa"),
    ///            "aaa".with_ref(|it| it.to_string()));
    /// ```
    #[const_trait]
    pub trait WithRef {
        /// Convert a reference with a closure
        #[inline(always)]
        fn with_ref<F, T>(&self, f: F) -> T
        where
            F: ~const FnOnce(&Self) -> T,
        {
            f(self)
        }
    }

    /// Trait to implement `with_ref_mut` on all types.
    ///
    /// # Examples
    ///
    /// ```
    /// # use upsilon_stdx::WithRefMut;
    /// # #[derive(Debug)]
    /// #[derive(Eq, PartialEq)]
    /// struct P(i32);
    /// let mut p = P(0);
    /// assert_eq!(1,
    ///            p.with_ref_mut(|it| {
    ///                 it.0 = 1;
    ///                 it.0
    ///             }));
    /// assert_eq!(P(1), p);
    /// ```
    #[const_trait]
    pub trait WithRefMut {
        /// Convert a mutable reference with a closure
        #[inline(always)]
        fn with_ref_mut<F, T>(&mut self, f: F) -> T
        where
            F: ~const FnOnce(&mut Self) -> T,
        {
            f(self)
        }
    }

    impl<T> With for T {}
    impl<T: ?Sized> WithRef for T {}
    impl<T: ?Sized> WithRefMut for T {}
}

mod take_if_unless {
    /// Defines `take_if` and `take_unless`, which return
    /// an option with the value depending on the
    /// condition
    ///
    /// # Examples
    ///
    /// ```
    /// # use upsilon_stdx::TakeIfUnless;
    /// assert_eq!(Some(1), 1.take_if(|&it| it > 0));
    /// assert_eq!(None, (-1).take_if(|&it| it > 0));
    /// ```
    ///
    /// Similarly with `take_unless`:
    ///
    /// ```
    /// # use upsilon_stdx::TakeIfUnless;
    /// assert_eq!(None, 1.take_unless(|&it| it > 0));
    /// assert_eq!(Some(-1), (-1).take_unless(|&it| it > 0));
    /// ```
    pub trait TakeIfUnless: Sized {
        /// Returns `Some(...)` if condition == true or None otherwise
        #[inline(always)]
        fn take_if<F>(self, condition: F) -> Option<Self>
        where
            F: FnOnce(&Self) -> bool,
        {
            condition(&self).then_some(self)
        }
        /// Returns `None` if condition == true or Some(...) otherwise
        #[inline(always)]
        fn take_unless<F>(self, condition: F) -> Option<Self>
        where
            F: FnOnce(&Self) -> bool,
        {
            Self::take_if(self, |it| !condition(it))
        }
    }

    impl<T> TakeIfUnless for T {}
}

mod take_if_unless_owned {
    /// Similar to [`TakeIfUnless`][`super::TakeIfUnless`], but works with the
    /// owned types
    pub trait TakeIfUnlessOwned: ToOwned {
        /// Similar to
        /// [`TakeIfUnless::take_if`][super::TakeIfUnless::take_if], but calls
        /// to_owned() too.
        #[inline(always)]
        fn take_if_owned<F>(&self, condition: F) -> Option<Self::Owned>
        where
            F: FnOnce(&Self) -> bool,
        {
            condition(self).then(|| self.to_owned())
        }
        /// Similar to
        /// [`TakeIfUnless::take_unless`][super::TakeIfUnless::take_unless],
        /// but calls to_owned() too.
        #[inline(always)]
        fn take_unless_owned<F>(&self, condition: F) -> Option<Self::Owned>
        where
            F: FnOnce(&Self) -> bool,
        {
            if condition(self) {
                None
            } else {
                Some(self.to_owned())
            }
        }
    }

    impl<T: ToOwned + ?Sized> TakeIfUnlessOwned for T {}
}

pub use also::*;
pub use clone_to::*;
pub use copy_to::*;
pub use take_if_unless::*;
pub use take_if_unless_owned::*;
pub use with::*;
