use futures::Stream;
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[pin_project]
pub struct ZipWithNext<S>
where
    S: Stream,
{
    #[pin]
    upstream: S,
    prev: Option<S::Item>,
}

impl<S> ZipWithNext<S>
where
    S: Stream,
{
    #[allow(missing_docs)]
    pub fn new(upstream: S) -> Self {
        Self {
            upstream,
            prev: None,
        }
    }
}

impl<S> Stream for ZipWithNext<S>
where
    S: Stream,
    S::Item: Clone,
{
    type Item = (S::Item, Option<S::Item>);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.upstream.poll_next(cx) {
            Poll::Ready(Some(current)) => match this.prev.replace(current.clone()) {
                Some(prev) => Poll::Ready(Some((prev, Some(current)))),
                None => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            },

            Poll::Ready(None) => match this.prev.take() {
                Some(prev) => Poll::Ready(Some((prev, None))),
                None => Poll::Ready(None),
            },

            // TODO Do we need to wake?
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{stream, StreamExt};

    #[tokio::test]
    async fn test_zip_with_next() {
        let numbers = ZipWithNext::new(stream::iter(0..0))
            .collect::<Vec<_>>()
            .await;
        assert_eq!(numbers, vec![]);

        let numbers = ZipWithNext::new(stream::iter(0..1))
            .collect::<Vec<_>>()
            .await;
        assert_eq!(numbers, vec![(0, None)]);

        let numbers = ZipWithNext::new(stream::iter(0..2))
            .collect::<Vec<_>>()
            .await;
        assert_eq!(numbers, vec![(0, Some(1)), (1, None)]);

        let numbers = ZipWithNext::new(stream::iter(0..4))
            .collect::<Vec<_>>()
            .await;
        assert_eq!(
            numbers,
            vec![(0, Some(1)), (1, Some(2)), (2, Some(3)), (3, None)]
        );
    }
}
