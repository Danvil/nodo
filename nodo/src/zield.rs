// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::future::Future;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;

#[inline]
pub async fn zield() {
    Zield(false).await
}

struct Zield(bool);

impl Future for Zield {
    type Output = ();

    // The futures executor is implemented as a FIFO queue, so all this future
    // does is re-schedule the future back to the end of the queue, giving room
    // for other futures to progress.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.0 {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
