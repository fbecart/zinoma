use futures::future::Either;
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
mod tests {
    use super::both;
    use async_std::task;
    use futures::future;

    #[test]
    fn both_are_true() {
        task::block_on(async {
            assert_eq!(true, both(future::ready(true), future::ready(true)).await)
        })
    }

    #[test]
    fn left_is_false() {
        task::block_on(async {
            assert_eq!(false, both(future::ready(false), future::ready(true)).await)
        })
    }

    #[test]
    fn right_is_false() {
        task::block_on(async {
            assert_eq!(false, both(future::ready(true), future::ready(false)).await)
        })
    }

    #[test]
    fn both_are_false() {
        task::block_on(async {
            assert_eq!(
                false,
                both(future::ready(false), future::ready(false)).await
            )
        })
    }
}
