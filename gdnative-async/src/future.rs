use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use crossbeam_channel::{Receiver, Sender};
use parking_lot::Mutex;

pub(crate) fn make<T>() -> (Yield<T>, Resume<T>) {
    let (arg_send, arg_recv) = crossbeam_channel::bounded(1);
    let waker = Arc::default();

    let future = Yield {
        waker: Arc::clone(&waker),
        arg_recv,
    };

    let resume = Resume { waker, arg_send };

    (future, resume)
}

/// Signal
pub struct Yield<T> {
    waker: Arc<Mutex<Option<Waker>>>,
    arg_recv: Receiver<T>,
}

impl<T: Send> Future for Yield<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.arg_recv.try_recv() {
            Ok(arg) => Poll::Ready(arg),
            Err(_) => {
                let mut waker = self.waker.lock();
                *waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

pub(crate) struct Resume<T> {
    waker: Arc<Mutex<Option<Waker>>>,
    arg_send: Sender<T>,
}

impl<T: Send> Resume<T> {
    /// Resume the task with a given argument from GDScript.
    pub fn resume(self, arg: T) {
        self.arg_send
            .send(arg)
            .expect("sender should not become disconnected");

        if let Some(waker) = self.waker.lock().take() {
            waker.wake();
        }
    }
}
