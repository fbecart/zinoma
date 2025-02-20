use futures::future::Either;
use futures::StreamExt;
use std::future::Future;

pub async fn both<L, R>(future1: L, future2: R) -> bool
where
    L: Future<Output = bool>,
    R: Future<Output = bool>,
{
    futures::pin_mut!(future1);
    futures::pin_mut!(future2);
    match futures::future::select(future1, future2).await {
        Either::Left((res1, future2)) => res1 && future2.await,
        Either::Right((res2, future1)) => res2 && future1.await,
    }
}

#[cfg(test)]
mod both_tests {
    use super::both;
    use async_std::task;
    use futures::future;

    #[test]
    fn both_are_true() {
        task::block_on(async {
            assert!(both(future::ready(true), future::ready(true)).await)
        })
    }

    #[test]
    fn left_is_false() {
        task::block_on(async {
            assert!(!(both(future::ready(false), future::ready(true)).await))
        })
    }

    #[test]
    fn right_is_false() {
        task::block_on(async {
            assert!(!(both(future::ready(true), future::ready(false)).await))
        })
    }

    #[test]
    fn both_are_false() {
        task::block_on(async {
            assert!(
                !(both(future::ready(false), future::ready(false)).await)
            )
        })
    }
}

/// Resolve as true unless any future in the iterator resolves as false.
pub async fn all<I>(i: I) -> bool
where
    I: IntoIterator,
    I::Item: Future<Output = bool>,
{
    let mut s = futures::stream::iter(i).buffer_unordered(64);
    while let Some(r) = s.next().await {
        if !r {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod all_tests {
    use super::all;
    use async_std::task;
    use future::Ready;
    use futures::future;

    #[test]
    fn true_for_empty() {
        task::block_on(async {
            let futures = Vec::<Ready<bool>>::new();
            assert!(all(futures).await);
        })
    }

    #[test]
    fn true_if_all_true() {
        task::block_on(async {
            let futures = vec![future::ready(true), future::ready(true)];
            assert!(all(futures).await);
        })
    }

    #[test]
    fn false_if_any_false() {
        task::block_on(async {
            assert!(
                !(all(vec![future::ready(false), future::ready(true)]).await)
            );
            assert!(
                !(all(vec![future::ready(true), future::ready(false)]).await)
            );
        })
    }
}
