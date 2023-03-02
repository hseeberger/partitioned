mod partitioned;
mod zip_with_next;

pub use crate::{partitioned::Partitioned, zip_with_next::ZipWithNext};

use std::{cell::RefCell, rc::Rc};

pub trait IteratorExt: Iterator
where
    Self: Sized,
{
    fn partitioned<F, K>(self, make_key: F) -> Partitioned<Self, F, K>
    where
        F: Fn(&Self::Item) -> K,
    {
        let upstream = Rc::new(RefCell::new(self.zip_with_next()));
        Partitioned::new(upstream, make_key)
    }

    fn zip_with_next(self) -> ZipWithNext<Self> {
        ZipWithNext::new(self)
    }
}

impl<I> IteratorExt for I where I: Iterator {}
