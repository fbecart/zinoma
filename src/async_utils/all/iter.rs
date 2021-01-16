use core::future::Future;
use futures::StreamExt;

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
    return true;
}

#[cfg(test)]
mod tests {
    use super::all;
    use async_std::task;
    use future::Ready;
    use futures::future;

    #[test]
    fn true_for_empty() {
        task::block_on(async {
            let futures = Vec::<Ready<bool>>::new();
            assert_eq!(true, all(futures).await);
        })
    }

    #[test]
    fn true_if_all_true() {
        task::block_on(async {
            let futures = vec![future::ready(true), future::ready(true)];
            assert_eq!(true, all(futures).await);
        })
    }

    #[test]
    fn false_if_any_false() {
        task::block_on(async {
            assert_eq!(
                false,
                all(vec![future::ready(false), future::ready(true)]).await
            );
            assert_eq!(
                false,
                all(vec![future::ready(true), future::ready(false)]).await
            );
        })
    }
}
