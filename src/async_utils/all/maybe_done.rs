use async_std::task::{Context, Poll};
use std::future::Future;
use std::pin::Pin;

pub enum MaybeDone<F: Future<Output = bool>> {
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
