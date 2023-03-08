pub struct ZipWithNext<I>
where
    I: Iterator,
{
    upstream: I,
    prev: Option<I::Item>,
}

impl<I> ZipWithNext<I>
where
    I: Iterator,
{
    #[allow(missing_docs)]
    pub fn new(upstream: I) -> Self {
        Self {
            upstream,
            prev: None,
        }
    }
}

impl<I> Iterator for ZipWithNext<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = (I::Item, Option<I::Item>);

    fn next(&mut self) -> Option<Self::Item> {
        match self.upstream.next() {
            Some(current) => match self.prev.take() {
                Some(prev) => {
                    self.prev = Some(current.clone());
                    Some((prev, Some(current)))
                }

                None => match self.upstream.next() {
                    Some(next) => {
                        self.prev = Some(next.clone());
                        Some((current, Some(next)))
                    }

                    None => Some((current, None)),
                },
            },

            None => self.prev.take().map(|last| (last, None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_with_next() {
        let numbers = 0..0;
        let numbers = ZipWithNext::new(numbers).collect::<Vec<_>>();
        assert_eq!(numbers, vec![]);

        let numbers = 0..1;
        let numbers = ZipWithNext::new(numbers).collect::<Vec<_>>();
        assert_eq!(numbers, vec![(0, None)]);

        let numbers = 0..2;
        let numbers = ZipWithNext::new(numbers).collect::<Vec<_>>();
        assert_eq!(numbers, vec![(0, Some(1)), (1, None)]);

        let numbers = 0..4;
        let numbers = ZipWithNext::new(numbers).collect::<Vec<_>>();
        assert_eq!(
            numbers,
            vec![(0, Some(1)), (1, Some(2)), (2, Some(3)), (3, None)]
        );
    }
}
