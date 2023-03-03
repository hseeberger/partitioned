mod partitioned;
mod zip_with_next;

pub use self::{partitioned::Partitioned, zip_with_next::ZipWithNext};
use std::{cell::RefCell, rc::Rc};

pub trait IteratorExt: Iterator
where
    Self: Sized,
{
    fn partitioned<F, K>(self, make_key: F) -> Partitioned<Self, F, K>
    where
        F: Fn(&Self::Item) -> K,
    {
        let upstream = Rc::new(RefCell::new(ZipWithNext::new(self)));
        Partitioned::new(upstream, make_key)
    }
}

impl<I> IteratorExt for I where I: Iterator {}
