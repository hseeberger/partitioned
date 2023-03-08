mod zip_with_next;

pub use self::zip_with_next::ZipWithNext;
use std::{cell::RefCell, fmt::Debug, mem::swap, rc::Rc};

pub trait IteratorExt: Iterator
where
    Self: Sized,
{
    fn partitioned<F, K>(self, make_key: F) -> Partitioned<Self, F, K>
    where
        F: Fn(&Self::Item) -> K,
    {
        Partitioned {
            upstream: Rc::new(RefCell::new(ZipWithNext::new(self))),
            make_key,
            key: None,
        }
    }
}

impl<I> IteratorExt for I where I: Iterator {}

pub struct Partitioned<I, F, K>
where
    I: Iterator,
{
    upstream: Rc<RefCell<ZipWithNext<I>>>,
    make_key: F,
    key: Option<K>,
}

impl<I, F, K> Iterator for Partitioned<I, F, K>
where
    I: Iterator,
    I::Item: Clone,
    F: Fn(&I::Item) -> K + Copy,
    K: Debug + Eq,
{
    type Item = Partition<I, F>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.upstream.borrow_mut().next() {
            Some((current, next)) => {
                let key = (self.make_key)(&current);
                if self.key.as_ref().map(|k| k == &key).unwrap_or_default() {
                    panic!("Partition with key `{key:?}` not consumed");
                }
                self.key.replace(key);

                Some(Partition {
                    upstream: self.upstream.clone(),
                    make_key: self.make_key,
                    current,
                    next,
                    terminated: false,
                })
            }

            None => None,
        }
    }
}

pub struct Partition<I, F>
where
    I: Iterator,
{
    upstream: Rc<RefCell<ZipWithNext<I>>>,
    make_key: F,
    current: I::Item,
    next: Option<I::Item>,
    terminated: bool,
}

impl<I, F, K> Iterator for Partition<I, F>
where
    I: Iterator,
    I::Item: Clone,
    F: Fn(&I::Item) -> K,
    K: Eq,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.terminated {
            return None;
        }

        match self.next {
            // If there is a next item with an equal key, `next()` must return `Some(current, next)`
            // with `current == self.next`. Hence we set `self.next` to the "next" `next`, set
            // `self.current` to the "next" `current` and emit the former `self.current`.
            Some(ref next) if (self.make_key)(&self.current) == (self.make_key)(next) => {
                match self.upstream.borrow_mut().next() {
                    Some((mut current, next)) => {
                        self.next = next;
                        swap(&mut self.current, &mut current);
                        Some(current) // The swapped one, i.e. the former self.current.
                    }

                    None => panic!("Impossible"),
                }
            }

            // If there is no next item or one with a different key, we set this partition
            // terminated and emit the current item as the final one.
            _ => {
                self.terminated = true;
                Some(self.current.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::iter::IteratorExt;

    #[test]
    fn test_partitioned() {
        let numbers = vec![1, 2, 2, 3, 3, 3, 4, 5, 5];
        let partitioned = numbers.into_iter().partitioned(|n| *n);
        let result = partitioned
            .map(|partition| partition.collect::<Vec<_>>())
            .collect::<Vec<_>>();
        assert_eq!(
            result,
            vec![vec![1], vec![2, 2], vec![3, 3, 3], vec![4], vec![5, 5]]
        );
    }

    #[test]
    #[should_panic = "Partition with key `1` not consumed"]
    fn test_partitioned_not_consumed() {
        let numbers = vec![1, 1, 2];
        let mut partitioned = numbers.into_iter().partitioned(|n| *n);

        let ones = partitioned.next();
        assert!(ones.is_some());

        partitioned.next();
    }
}
