use std::time::Duration;

pub async fn sleep(d: Duration) {
    tokio::time::sleep(d).await;
}

/// iterator over the gaps between neighboring elements in an iterator
pub struct Gaps<It>
where It: Iterator {
    inner: It,
    prev: Option<It::Item>,
}

impl<It> Iterator for Gaps<It>
where It: Iterator,
      It::Item: Clone {
    type Item = (Option<It::Item>, Option<It::Item>);
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.inner.next();
        if next.is_none() && self.prev.is_none() {
            None
        } else {
            Some((std::mem::replace(&mut self.prev, next.clone()), next))
        }
    }
}

impl<It> Gaps<It>
where It: Iterator {
    pub fn new<I>(it: I) -> Self
    where I: IntoIterator<IntoIter = It> {
        Self { inner: it.into_iter(), prev: None }
    }
}
