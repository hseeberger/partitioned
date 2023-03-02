use std::{cell::RefCell, mem::swap, rc::Rc};

pub trait IteratorExt: Iterator
where
    Self: Sized,
{
    fn partitioned<F, K>(self, make_key: F) -> Partitioned<Self, F, K>
    where
        F: Fn(&Self::Item) -> K,
    {
        let upstream = Rc::new(RefCell::new(self.zip_with_next()));
        Partitioned {
            upstream,
            make_key,
            key: None,
        }
    }

    fn zip_with_next(self) -> ZipWithNext<Self> {
        ZipWithNext {
            upstream: self,
            current: None,
        }
    }
}

impl<I> IteratorExt for I where I: Iterator {}

pub struct ZipWithNext<I>
where
    I: Iterator,
{
    upstream: I,
    current: Option<I::Item>,
}

impl<I> Iterator for ZipWithNext<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = (I::Item, Option<I::Item>);

    fn next(&mut self) -> Option<Self::Item> {
        match self.upstream.next() {
            Some(item) => match self.current.take() {
                Some(current) => {
                    self.current = Some(item.clone());
                    Some((current, Some(item)))
                }

                None => match self.upstream.next() {
                    Some(next) => {
                        self.current = Some(next.clone());
                        Some((item, Some(next)))
                    }

                    None => Some((item, None)),
                },
            },

            None => self.current.take().map(|last| (last, None)),
        }
    }
}

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
    K: Eq,
{
    type Item = Partition<I, F>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.upstream.borrow_mut().next() {
            Some((current, next)) => {
                let key = Some((self.make_key)(&current));
                if key == self.key {
                    panic!("Partition not consumed")
                } else {
                    self.key = key
                };

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

            _ => {
                self.terminated = true;
                Some(self.current.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn test_zip_with_next() {
        let numbers = 0..0;
        let numbers = numbers.zip_with_next().collect::<Vec<_>>();
        assert_eq!(numbers, vec![]);

        let numbers = 0..1;
        let numbers = numbers.zip_with_next().collect::<Vec<_>>();
        assert_eq!(numbers, vec![(0, None)]);

        let numbers = 0..2;
        let numbers = numbers.zip_with_next().collect::<Vec<_>>();
        assert_eq!(numbers, vec![(0, Some(1)), (1, None)]);

        let numbers = 0..4;
        let numbers = numbers.zip_with_next().collect::<Vec<_>>();
        assert_eq!(
            numbers,
            vec![(0, Some(1)), (1, Some(2)), (2, Some(3)), (3, None)]
        );
    }

    #[test]
    fn test_partitioned() {
        let numbers: Vec<i32> = vec![1, 2, 2, 3, 3, 3, 4, 5, 5];
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
    #[should_panic = "Partition not consumed"]
    fn test_partitioned_not_consumed() {
        let numbers: Vec<i32> = vec![1, 1, 2];
        let mut partitioned = numbers.into_iter().partitioned(|n| *n);

        let ones = partitioned.next();
        assert!(ones.is_some());

        partitioned.next();
    }
}
