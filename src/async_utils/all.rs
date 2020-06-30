use crate::task::{Context, Poll};
use pin_project_lite::pin_project;
use std::future::Future;
use std::pin::Pin;

pub fn all<L, R>(left: L, right: R) -> impl Future<Output = bool>
where
    L: Future<Output = bool> + Sized,
    R: Future<Output = bool> + Sized,
{
    All {
        left: MaybeDone::Future(left),
        right: MaybeDone::Future(right),
    }
}

pin_project! {
    pub struct All<L, R>
    where
        L: Future<Output = bool>,
        R: Future<Output = bool>,
    {
        #[pin] left: MaybeDone<L>,
        #[pin] right: MaybeDone<R>,
    }
}

impl<L, R> Future for All<L, R>
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

enum MaybeDone<F: Future<Output = bool>> {
    Future(F),
    Done(bool),
}

impl<F: Future<Output = bool>> Future for MaybeDone<F> {
    type Output = bool;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let res = unsafe {
            match Pin::as_mut(&mut self).get_unchecked_mut() {
                MaybeDone::Future(future) => match Pin::new_unchecked(future).poll(cx) {
                    Poll::Ready(res) => res,
                    Poll::Pending => return Poll::Pending,
                },
                MaybeDone::Done(res) => return Poll::Ready(*res),
            }
        };
        self.set(MaybeDone::Done(res));
        Poll::Ready(res)
    }
}

#[cfg(test)]
mod tests {
    use super::all;
    use async_std::task;
    use futures::future;

    #[test]
    fn both_are_true() {
        assert!(task::block_on(async {
            all(future::ready(true), future::ready(true)).await
        }))
    }

    #[test]
    fn left_is_false() {
        assert!(!task::block_on(async {
            all(future::ready(false), future::ready(true)).await
        }))
    }

    #[test]
    fn right_is_false() {
        assert!(!task::block_on(async {
            all(future::ready(true), future::ready(false)).await
        }))
    }

    #[test]
    fn both_are_false() {
        assert!(!task::block_on(async {
            all(future::ready(false), future::ready(false)).await
        }))
    }
}
