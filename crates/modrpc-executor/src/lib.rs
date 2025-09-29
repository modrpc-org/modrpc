#![cfg_attr(not(feature = "std"), no_std)]

use core::{
    future::Future,
    pin::Pin,
    time::Duration,
    task::{Context, Poll},
};
#[cfg(feature = "std")]
use std::rc::Rc;

pub trait ModrpcExecutor {
    type Sleep: Future<Output = ()> + 'static;
    type Interval: Interval + 'static;
    type Sleeper: Sleeper + 'static;

    fn new() -> Self;
    fn spawner(&mut self) -> ispawn::LocalSpawner;
    fn run_until<R>(&mut self, future: impl Future<Output = R>) -> R;
    fn sleep(duration: Duration) -> Self::Sleep;
    fn interval(period: Duration) -> Self::Interval;
    fn new_sleeper() -> Self::Sleeper;
}

pub trait Interval {
    #[allow(async_fn_in_trait)]
    async fn tick(&mut self);
}

/// A dyn-compatible interface for polling one sleep `Future` at a time.
pub trait Sleeper {
    #[allow(async_fn_in_trait)]
    fn snooze(self: Pin<&mut Self>, duration: Duration) -> bool;

    #[allow(async_fn_in_trait)]
    fn poll_sleep(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()>;
}

#[cfg(feature = "async-timer")]
impl Interval for async_timer::Interval {
    async fn tick(&mut self) {
        self.await;
    }
}

#[cfg(feature = "futures-executor")]
pub struct FuturesExecutor {
    local_pool: futures_executor::LocalPool,
}

#[cfg(feature = "futures-executor")]
impl ModrpcExecutor for FuturesExecutor {
    type Sleep = async_timer::timer::Platform;
    type Interval = async_timer::Interval;
    type Sleeper = AsyncTimerSleeper;

    fn new() -> Self {
        let local_pool = futures_executor::LocalPool::new();
        Self { local_pool }
    }

    fn spawner(&mut self) -> ispawn::LocalSpawner {
        ispawn::LocalSpawner::new(Rc::new(self.local_pool.spawner()))
    }

    fn run_until<R>(&mut self, future: impl Future<Output = R>) -> R {
        self.local_pool.run_until(future)
    }

    fn sleep(duration: Duration) -> Self::Sleep {
        async_timer::timer::Platform::new(duration)
    }

    fn interval(period: Duration) -> Self::Interval {
        async_timer::Interval::new(period)
    }

    fn new_sleeper() -> Self::Sleeper {
        AsyncTimerSleeper { current_sleep: None }
    }
}

#[cfg(feature = "tokio")]
pub struct TokioExecutor {
    rt: tokio::runtime::Runtime,
    local_set: Rc<tokio::task::LocalSet>,
}

#[cfg(feature = "tokio")]
impl TokioExecutor {
    pub fn tokio_runtime(&self) -> &tokio::runtime::Runtime {
        &self.rt
    }

    pub fn local_set(&self) -> &tokio::task::LocalSet {
        &self.local_set
    }
}

#[cfg(feature = "tokio")]
impl ModrpcExecutor for TokioExecutor {
    type Sleep = tokio::time::Sleep;
    type Interval = tokio::time::Interval;
    type Sleeper = TokioSleeper;

    fn new() -> Self {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("new modrpc tokio runtime");
        let local_set = Rc::new(tokio::task::LocalSet::new());

        Self { rt, local_set }
    }

    fn spawner(&mut self) -> ispawn::LocalSpawner {
        ispawn::LocalSpawner::new(self.local_set.clone())
    }

    fn run_until<R>(&mut self, future: impl Future<Output = R>) -> R {
        self.local_set.block_on(&self.rt, future)
    }

    fn sleep(duration: Duration) -> Self::Sleep {
        tokio::time::sleep(duration)
    }

    fn interval(period: Duration) -> Self::Interval {
        tokio::time::interval(period)
    }

    fn new_sleeper() -> Self::Sleeper {
        TokioSleeper { current_sleep: None }
    }
}

#[cfg(feature = "tokio")]
impl Interval for tokio::time::Interval {
    async fn tick(&mut self) {
        let _ = self.tick().await;
    }
}

#[cfg(feature = "tokio")]
pub struct TokioSleeper {
    current_sleep: Option<tokio::time::Sleep>,
}

#[cfg(feature = "tokio")]
impl TokioSleeper {
    fn current_sleep(self: Pin<&mut Self>) -> Option<Pin<&mut tokio::time::Sleep>> {
        unsafe { self.map_unchecked_mut(|s| &mut s.current_sleep) }.as_pin_mut()
    }
}

#[cfg(feature = "tokio")]
impl Sleeper for TokioSleeper {
    fn snooze(self: Pin<&mut Self>, duration: Duration) -> bool {
        // SAFETY: We don't overwrite an existing `Sleep` object.
        let current_sleep = &mut unsafe { self.get_unchecked_mut() }.current_sleep;
        if current_sleep.is_none() {
            *current_sleep = Some(tokio::time::sleep(duration));
            true
        } else {
            false
        }
    }

    fn poll_sleep(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(sleep) = self.as_mut().current_sleep() {
            match sleep.poll(cx) {
                Poll::Ready(()) => {
                    // SAFETY: There are no dangling references to the inner `Sleep` object after
                    // dropping it.
                    unsafe { self.get_unchecked_mut() }.current_sleep = None;
                    Poll::Ready(())
                }
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(())
        }
    }
}

#[cfg(feature = "dioxus")]
pub use ispawn::DioxusSpawner;

#[cfg(feature = "dioxus")]
pub struct DioxusExecutor;

#[cfg(feature = "dioxus")]
impl ModrpcExecutor for DioxusExecutor {
    type Sleep = async_timer::timer::Platform;
    type Interval = async_timer::Interval;
    type Sleeper = AsyncTimerSleeper;

    fn new() -> Self { Self }

    fn spawner(&mut self) -> ispawn::LocalSpawner {
        ispawn::LocalSpawner::new(ispawn::DioxusSpawner)
    }

    fn run_until<R>(&mut self, _: impl Future<Output = R>) -> R {
        panic!("DioxusExecutor is only supported in single-threaded modrpc runtimes");
    }

    fn sleep(duration: Duration) -> Self::Sleep {
        async_timer::timer::Platform::new(duration)
    }

    fn interval(period: Duration) -> Self::Interval {
        async_timer::Interval::new(period)
    }

    fn new_sleeper() -> Self::Sleeper {
        AsyncTimerSleeper { current_sleep: None }
    }
}

#[cfg(feature = "wasm-bindgen")]
pub use ispawn::WasmBindgenSpawner;

#[cfg(feature = "wasm-bindgen")]
pub struct WasmBindgenExecutor;

#[cfg(feature = "wasm-bindgen")]
impl ModrpcExecutor for WasmBindgenExecutor {
    type Sleep = async_timer::timer::Platform;
    type Interval = async_timer::Interval;
    type Sleeper = AsyncTimerSleeper;

    fn new() -> Self { Self }

    fn spawner(&mut self) -> ispawn::LocalSpawner {
        ispawn::LocalSpawner::new(ispawn::WasmBindgenSpawner)
    }

    fn run_until<R>(&mut self, _: impl Future<Output = R>) -> R {
        panic!("WasmBindgenExecutor is only supported in single-threaded modrpc runtimes");
    }

    fn sleep(duration: Duration) -> Self::Sleep {
        async_timer::timer::Platform::new(duration)
    }

    fn interval(period: Duration) -> Self::Interval {
        async_timer::Interval::new(period)
    }

    fn new_sleeper() -> Self::Sleeper {
        AsyncTimerSleeper { current_sleep: None }
    }
}

#[cfg(any(
    feature = "dioxus",
    feature = "futures-executor",
    feature = "wasm-bindgen",
))]
pub struct AsyncTimerSleeper {
    current_sleep: Option<async_timer::timer::Platform>,
}

#[cfg(any(
    feature = "dioxus",
    feature = "futures-executor",
    feature = "wasm-bindgen",
))]
impl AsyncTimerSleeper {
    fn current_sleep(self: Pin<&mut Self>) -> Option<Pin<&mut async_timer::timer::Platform>> {
        unsafe { self.map_unchecked_mut(|s| &mut s.current_sleep) }.as_pin_mut()
    }
}

#[cfg(any(
    feature = "dioxus",
    feature = "futures-executor",
    feature = "wasm-bindgen",
))]
impl Sleeper for AsyncTimerSleeper {
    fn snooze(self: Pin<&mut Self>, duration: Duration) -> bool {
        // SAFETY: We don't overwrite an existing `Sleep` object.
        let current_sleep = &mut unsafe { self.get_unchecked_mut() }.current_sleep;
        if current_sleep.is_none() {
            *current_sleep = Some(async_timer::timer::Platform::new(duration));
            true
        } else {
            false
        }
    }

    fn poll_sleep(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(sleep) = self.as_mut().current_sleep() {
            match sleep.poll(cx) {
                Poll::Ready(()) => {
                    // SAFETY: There are no dangling references to the inner `Sleep` object after
                    // dropping it.
                    unsafe { self.get_unchecked_mut() }.current_sleep = None;
                    Poll::Ready(())
                }
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(())
        }
    }
}
