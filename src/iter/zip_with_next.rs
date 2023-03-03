pub struct ZipWithNext<I>
where
    I: Iterator,
{
    upstream: I,
    current: Option<I::Item>,
}

impl<I> ZipWithNext<I>
where
    I: Iterator,
{
    pub(crate) fn new(upstream: I) -> Self {
        Self {
            upstream,
            current: None,
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
