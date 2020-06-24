use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::Delay;

pub struct Receiver<T> {
    inner: crossbeam_channel::Receiver<T>,
    delay: Duration,
    pending: Option<Delay>,
}

impl<T> Receiver<T> {
    pub fn new(r: crossbeam_channel::Receiver<T>, delay: Duration) -> Receiver<T> {
        Receiver {
            inner: r,
            delay: delay,
            pending: None,
        }
    }

    // pub fn into_inner(self) -> crossbeam_channel::Receiver<T> {
    //     self.inner
    // }
}

impl<T> Future for Receiver<T> {
    type Output = Result<T, crossbeam_channel::TryRecvError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {
            inner,
            delay,
            pending,
        } = unsafe { self.get_unchecked_mut() };
        loop {
            match pending {
                None => match inner.try_recv() {
                    Err(crossbeam_channel::TryRecvError::Empty) => {
                        *pending = Some(tokio::time::delay_for(*delay));
                    }
                    result => return Poll::Ready(result),
                },
                Some(pending_value) => {
                    let pin_pending = unsafe { Pin::new_unchecked(pending_value) };
                    futures::ready!(pin_pending.poll(cx));
                    *pending = None;
                }
            }
        }
    }
}
