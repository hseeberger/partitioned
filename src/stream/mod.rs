mod zip_with_next;

use self::zip_with_next::ZipWithNext;
use futures::{Stream, StreamExt as FuturesStreamExt};
use parking_lot::RwLock;
use pin_project::pin_project;
use std::{
    fmt::Debug,
    mem::swap,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

pub trait StreamExt: Stream
where
    Self: Sized,
{
    fn partitioned<F, K>(self, make_key: F) -> Partitioned<Self, F, K>
    where
        F: Fn(&Self::Item) -> K,
    {
        Partitioned {
            upstream: Arc::new(RwLock::new(ZipWithNext::new(self))),
            make_key,
            key: None,
        }
    }
}

impl<S> StreamExt for S where S: Stream {}

#[pin_project]
pub struct Partitioned<S, F, K>
where
    S: Stream,
{
    upstream: Arc<RwLock<ZipWithNext<S>>>,
    make_key: F,
    key: Option<K>,
}

impl<S, F, K> Stream for Partitioned<S, F, K>
where
    S: Stream + Unpin,
    S::Item: Clone,
    F: Fn(&S::Item) -> K + Copy,
    K: Debug + Eq,
{
    type Item = Partition<S, F>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.upstream.write().poll_next_unpin(cx) {
            Poll::Ready(Some((current, next))) => {
                let key = (this.make_key)(&current);
                if this.key.as_ref().map(|k| k == &key).unwrap_or_default() {
                    panic!("Partition with key `{key:?}` not consumed");
                }
                this.key.replace(key);

                Poll::Ready(Some(Partition {
                    upstream: this.upstream.clone(),
                    make_key: *this.make_key,
                    current,
                    next,
                    terminated: false,
                }))
            }

            Poll::Ready(None) => Poll::Ready(None),

            Poll::Pending => Poll::Pending,
        }
    }
}

#[pin_project]
pub struct Partition<S, F>
where
    S: Stream,
{
    upstream: Arc<RwLock<ZipWithNext<S>>>,
    make_key: F,
    current: S::Item,
    next: Option<S::Item>,
    terminated: bool,
}

impl<S, F, K> Stream for Partition<S, F>
where
    S: Stream + Unpin,
    S::Item: Clone,
    F: Fn(&S::Item) -> K,
    K: Eq,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        if *this.terminated {
            return Poll::Ready(None);
        }

        match this.next {
            // If there is a next item with an equal key, `poll_next_unpin()` must eventually return
            // `Some(current, next)` with `current == this.next`. Hence we set `this.next` to the
            // "next" `next`, set `this.current` to the "next" `current` and emit the former
            // `this.current`.
            Some(ref next) if (this.make_key)(this.current) == (this.make_key)(next) => {
                match this.upstream.write().poll_next_unpin(cx) {
                    Poll::Ready(Some((mut current, next))) => {
                        *this.next = next;
                        swap(this.current, &mut current);
                        Poll::Ready(Some(current)) // The swapped one, i.e. the former this.current.
                    }

                    Poll::Ready(None) => panic!("Impossible"),

                    Poll::Pending => Poll::Pending,
                }
            }

            // If there is no next item or one with a different key, we set this partition
            // terminated and emit the current item as the final one.
            _ => {
                *this.terminated = true;
                Poll::Ready(Some(this.current.clone()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::stream::StreamExt;
    use futures::stream::{self, StreamExt as FuturesStreamExt};

    #[tokio::test]
    async fn test_partitioned() {
        let numbers = stream::iter(vec![1, 2, 2, 3, 3, 3, 4, 5, 5]);
        let partitioned = numbers.partitioned(|n| *n);
        let result = partitioned
            .then(|partition| partition.collect::<Vec<_>>())
            .collect::<Vec<_>>()
            .await;
        assert_eq!(
            result,
            vec![vec![1], vec![2, 2], vec![3, 3, 3], vec![4], vec![5, 5]]
        );
    }

    #[tokio::test]
    #[should_panic = "Partition with key `1` not consumed"]
    async fn test_partitioned_not_consumed() {
        let numbers = stream::iter(vec![1, 1, 2]);
        let mut partitioned = numbers.partitioned(|n| *n);

        let ones = partitioned.next().await;
        assert!(ones.is_some());

        partitioned.next().await;
    }
}
