use super::maybe_done::MaybeDone;
use crate::task::{Context, Poll};
use pin_project_lite::pin_project;
use std::future::Future;
use std::pin::Pin;

pub fn both<L, R>(left: L, right: R) -> impl Future<Output = bool>
where
    L: Future<Output = bool> + Sized,
    R: Future<Output = bool> + Sized,
{
    Both {
        left: MaybeDone::Future(left),
        right: MaybeDone::Future(right),
    }
}

pin_project! {
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Both<L, R>
    where
        L: Future<Output = bool>,
        R: Future<Output = bool>,
    {
        #[pin] left: MaybeDone<L>,
        #[pin] right: MaybeDone<R>,
    }
}

impl<L, R> Future for Both<L, R>
where
    L: Future<Output = bool>,
    R: Future<Output = bool>,
{
    type Output = bool;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        let left_poll = Pin::new(&mut this.left).poll(cx);
        let right_poll = Pin::new(&mut this.right).poll(cx);

        match (left_poll, right_poll) {
            (Poll::Ready(true), Poll::Ready(true)) => Poll::Ready(true),
            (Poll::Ready(false), _) | (_, Poll::Ready(false)) => Poll::Ready(false),
            _ => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::both;
    use async_std::task;
    use futures::future;

    #[test]
    fn both_are_true() {
        assert!(task::block_on(async {
            both(future::ready(true), future::ready(true)).await
        }))
    }

    #[test]
    fn left_is_false() {
        assert!(!task::block_on(async {
            both(future::ready(false), future::ready(true)).await
        }))
    }

    #[test]
    fn right_is_false() {
        assert!(!task::block_on(async {
            both(future::ready(true), future::ready(false)).await
        }))
    }

    #[test]
    fn both_are_false() {
        assert!(!task::block_on(async {
            both(future::ready(false), future::ready(false)).await
        }))
    }
}
