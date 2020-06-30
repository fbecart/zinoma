use core::future::Future;
use core::iter::FromIterator;
use core::pin::Pin;
use core::task::{Context, Poll};

use super::maybe_done::MaybeDone;

pub fn all<I>(i: I) -> All<I::Item>
where
    I: IntoIterator,
    I::Item: Future<Output = bool>,
{
    let elems: Box<[_]> = i.into_iter().map(MaybeDone::Future).collect();
    All {
        elems: elems.into(),
    }
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct All<F: Future<Output = bool>> {
    elems: Pin<Box<[MaybeDone<F>]>>,
}

impl<F> Future for All<F>
where
    F: Future<Output = bool>,
{
    type Output = bool;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut all_done = true;

        for elem in iter_pin_mut(self.elems.as_mut()) {
            match elem.poll(cx) {
                Poll::Ready(false) => return Poll::Ready(false),
                Poll::Pending => all_done = false,
                _ => {}
            }
        }

        if all_done {
            Poll::Ready(true)
        } else {
            Poll::Pending
        }
    }
}

impl<F: Future<Output = bool>> FromIterator<F> for All<F> {
    fn from_iter<T: IntoIterator<Item = F>>(iter: T) -> Self {
        all(iter)
    }
}

fn iter_pin_mut<T>(slice: Pin<&mut [T]>) -> impl Iterator<Item = Pin<&mut T>> {
    // Safety: `std` _could_ make this unsound if it were to decide Pin's
    // invariants aren't required to transmit through slices. Otherwise this has
    // the same safety as a normal field pin projection.
    unsafe { slice.get_unchecked_mut() }
        .iter_mut()
        .map(|t| unsafe { Pin::new_unchecked(t) })
}
