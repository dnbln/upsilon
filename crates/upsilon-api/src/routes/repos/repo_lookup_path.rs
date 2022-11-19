use std::ops::Index;

use rocket::http::uri::fmt::Path;
use rocket::http::uri::Segments;
use rocket::request::{FromParam, FromSegments};

use upsilon_models::namespace::{PlainNamespaceFragment, PlainNamespaceFragmentRef};

const LOOKUP_PATH_SEGMENT_SEPARATOR: char = '.';

pub struct RepoLookupPath {
    path: Vec<PlainNamespaceFragment>,
}

impl RepoLookupPath {
    fn from_iter<T, I>(iter: I) -> Result<Self, NsLookupPathError>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
        T: Into<PlainNamespaceFragment>,
    {
        let iter = iter.into_iter();

        if iter.len() == 0 {
            return Err(NsLookupPathError::Empty);
        }

        if iter.len() > 3 {
            return Err(NsLookupPathError::TooManySegments);
        }

        Ok(RepoLookupPath {
            path: iter.map(Into::into).collect(),
        })
    }

    pub fn len(&self) -> usize {
        self.path.len()
    }

    pub fn last(&self) -> PlainNamespaceFragmentRef {
        self.path[self.len() - 1].as_ref()
    }
}

impl Index<usize> for RepoLookupPath {
    type Output = PlainNamespaceFragment;

    fn index(&self, index: usize) -> &Self::Output {
        &self.path[index]
    }
}

#[derive(Debug)]
pub enum NsLookupPathError {
    Empty,
    TooManySegments,
}

impl<'r> FromSegments<'r> for RepoLookupPath {
    type Error = NsLookupPathError;

    fn from_segments(segments: Segments<'r, Path>) -> Result<Self, Self::Error> {
        struct SegmentsWrapper<'r>(Segments<'r, Path>);

        impl<'r> Iterator for SegmentsWrapper<'r> {
            type Item = &'r str;

            fn next(&mut self) -> Option<Self::Item> {
                self.0.next()
            }
        }

        impl<'r> ExactSizeIterator for SegmentsWrapper<'r> {
            fn len(&self) -> usize {
                self.0.len()
            }
        }

        Self::from_iter(SegmentsWrapper(segments))
    }
}

impl<'r> FromParam<'r> for RepoLookupPath {
    type Error = NsLookupPathError;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        Self::from_iter(
            param
                .split(LOOKUP_PATH_SEGMENT_SEPARATOR)
                .collect::<Vec<_>>(),
        )
    }
}
