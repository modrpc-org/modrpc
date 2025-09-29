use std::{
    cell::Cell,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

pub enum FlushBatcherStatus {
    Snooze { duration: Duration },
    FlushNow,
    DoNotFlush,
}

#[derive(Copy, Clone)]
enum NextFlushTime {
    Instant(Instant),
    Immediate,
}

#[derive(Clone)]
pub struct FlushBatcher {
    inner: Rc<Inner>,
}

struct Inner {
    max_flush_delay: Duration,
    next_flush_time: Cell<Option<NextFlushTime>>,
    waker: Cell<Option<Waker>>,
}

impl FlushBatcher {
    pub fn new(max_flush_delay: Duration) -> Self {
        Self {
            inner: Rc::new(Inner {
                max_flush_delay,
                next_flush_time: Cell::new(None),
                waker: Cell::new(None),
            }),
        }
    }

    pub fn schedule_flush(&self) {
        if self.inner.next_flush_time.get().is_some() {
            return;
        }

        if self.inner.max_flush_delay.is_zero() {
            self.inner
                .next_flush_time
                .set(Some(NextFlushTime::Immediate));
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            self.inner.next_flush_time.set(
                Instant::now()
                    .checked_add(self.inner.max_flush_delay)
                    .map(NextFlushTime::Instant),
            );
            #[cfg(target_arch = "wasm32")]
            self.inner
                .next_flush_time
                .set(Some(NextFlushTime::Immediate));
        }

        if let Some(waker) = self.inner.waker.take() {
            waker.wake();
        }
    }

    pub fn cancel_flush(&self) {
        self.inner.next_flush_time.set(None);
    }

    pub fn handle_flush(&self) -> FlushBatcherStatus {
        if let Some(next_flush_time) = self.inner.next_flush_time.get() {
            match next_flush_time {
                NextFlushTime::Immediate => {
                    self.inner.next_flush_time.set(None);
                    FlushBatcherStatus::FlushNow
                }
                NextFlushTime::Instant(next_flush_time) => {
                    let now = Instant::now();
                    if now >= next_flush_time {
                        self.inner.next_flush_time.set(None);
                        FlushBatcherStatus::FlushNow
                    } else {
                        FlushBatcherStatus::Snooze {
                            duration: next_flush_time - now,
                        }
                    }
                }
            }
        } else {
            FlushBatcherStatus::DoNotFlush
        }
    }

    pub fn wait(&self) -> FlushBatcherWait {
        FlushBatcherWait {
            inner: self.inner.clone(),
        }
    }
}

pub struct FlushBatcherWait {
    inner: Rc<Inner>,
}

impl Future for FlushBatcherWait {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.inner.next_flush_time.get().is_some() {
            return Poll::Ready(());
        }

        self.inner.waker.set(Some(cx.waker().clone()));

        Poll::Pending
    }
}
