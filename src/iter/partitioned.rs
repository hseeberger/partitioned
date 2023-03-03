use super::ZipWithNext;
use std::{cell::RefCell, mem::swap, rc::Rc};

pub struct Partitioned<I, F, K>
where
    I: Iterator,
{
    upstream: Rc<RefCell<ZipWithNext<I>>>,
    make_key: F,
    key: Option<K>,
}

impl<I, F, K> Partitioned<I, F, K>
where
    I: Iterator,
{
    pub(crate) fn new(upstream: Rc<RefCell<ZipWithNext<I>>>, make_key: F) -> Self {
        Partitioned {
            upstream,
            make_key,
            key: None,
        }
    }
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
    use crate::iter::IteratorExt;

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
