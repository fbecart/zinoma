use async_std::stream::Stream;
use smol::Timer;
use std::cmp;
use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

#[derive(Debug)]
pub struct ExponentialBackoff {
    delay: Timer,
    max_interval: Duration,
    current_interval: Duration,
    interval_increase_factor: f32,
}

impl ExponentialBackoff {
    pub fn new(
        start_interval: Duration,
        max_interval: Duration,
        interval_increase_factor: f32,
    ) -> Self {
        Self {
            delay: Timer::after(start_interval),
            max_interval,
            current_interval: start_interval,
            interval_increase_factor,
        }
    }
}

impl Stream for ExponentialBackoff {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if Pin::new(&mut self.delay).poll(cx).is_pending() {
            return Poll::Pending;
        }

        let next_interval = cmp::min(
            self.current_interval.mul_f32(self.interval_increase_factor),
            self.max_interval,
        );

        self.current_interval = next_interval;
        let _ = mem::replace(&mut self.delay, Timer::after(next_interval));
        Poll::Ready(Some(()))
    }
}
