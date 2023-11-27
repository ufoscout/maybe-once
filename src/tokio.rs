use std::ops::Deref;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;

use core::pin::Pin;
use parking_lot::{Mutex, RwLock};
use std::future::Future;
use tokio::sync::{RwLock as AsyncRwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct MaybeSingleAsync<T> {
    data: Arc<RwLock<Option<Arc<T>>>>,
    lock_mutex: Arc<AsyncRwLock<()>>,
    init: fn() -> Pin<Box<dyn Send + Future<Output = T>>>,
    callers: Arc<Mutex<AtomicUsize>>,
}

impl<T> MaybeSingleAsync<T> {
    pub fn new(init: fn() -> Pin<Box<dyn Send + Future<Output = T>>>) -> Self {
        MaybeSingleAsync {
            data: Arc::new(RwLock::new(None)),
            init,
            lock_mutex: Arc::new(AsyncRwLock::new(())),
            callers: Arc::new(Mutex::new(AtomicUsize::new(0))),
        }
    }

    pub async fn data(&self, serial: bool) -> Data<'_, T> {
        {
            let lock = self.callers.lock();
            let callers = lock.load(SeqCst) + 1;
            lock.store(callers, SeqCst);
        }

        let data_arc = {
            let is_none = { self.data.read().is_none() };

            if is_none {
                let _lock_mutex = self.lock_mutex.write().await;

                let is_none = { self.data.read().is_none() };

                if is_none {
                    let init = { (self.init)().await };
                    let mut write_lock = self.data.write();
                    if write_lock.is_none() {
                        //  println!("--- INIT ---");
                        *write_lock = Some(Arc::new(init));
                    }
                }
            };

            let lock = self.data.read();
            //println!("---- Exec {}", rnd);
            match lock.as_ref() {
                Some(data) => data.clone(),
                None => panic!("There should always be data here!"),
            }
        };

        let (read_lock, write_lock) = if serial {
            (None, Some(self.lock_mutex.write().await))
        } else {
            (Some(self.lock_mutex.read().await), None)
        };

        Data {
            data_arc,
            data: self.data.clone(),
            callers: self.callers.clone(),
            read_lock,
            write_lock,
        }
    }
}

pub struct Data<'a, T> {
    data_arc: Arc<T>,
    data: Arc<RwLock<Option<Arc<T>>>>,
    #[allow(dead_code)]
    read_lock: Option<RwLockReadGuard<'a, ()>>,
    #[allow(dead_code)]
    write_lock: Option<RwLockWriteGuard<'a, ()>>,
    callers: Arc<Mutex<AtomicUsize>>,
}

impl<'a, T> Drop for Data<'a, T> {
    fn drop(&mut self) {
        //println!("--- Dropping DATA ---");
        let lock = self.callers.lock();
        let callers = lock.load(SeqCst) - 1;
        lock.store(callers, SeqCst);

        if callers == 0 {
            println!("MaybeSingle --- Dropping DATA ---");
            let mut data = self.data.write();
            *data = None;
        }
    }
}

impl<'a, T> Deref for Data<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data_arc.as_ref()
    }
}

impl<'a, T> AsRef<T> for Data<'a, T> {
    fn as_ref(&self) -> &T {
        self.data_arc.as_ref()
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use rand::{thread_rng, Rng};
    use std::time::Duration;
    use tokio::sync::Mutex as AsyncMutex;

    #[test]
    fn maybe_should_be_send() {
        let maybe = MaybeSingleAsync::new(|| Box::pin(async {}));
        need_send(maybe);
    }

    fn need_send<T: Send>(_t: T) {}
    fn need_sync<T: Sync>(_t: T) {}

    #[test]
    fn maybe_should_be_sync() {
        let maybe = MaybeSingleAsync::new(|| Box::pin(async {}));
        need_sync(maybe);
    }

    #[tokio::test]
    async fn async_should_execute_in_parallel() {
        let maybe = MaybeSingleAsync::new(|| Box::pin(async {}));
        let maybe = Arc::new(maybe);

        let responses = Arc::new(AsyncMutex::new(vec![]));

        let mut handles = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            let responses = responses.clone();
            let sleep_for = thread_rng().gen_range(0..1000);
            handles.push(tokio::spawn(async move {
                let _data = maybe.data(false).await;
                assert!(maybe.data.read().is_some());
                println!(" exec {} start", i);
                tokio::time::sleep(Duration::from_nanos(sleep_for)).await;
                println!(" exec {} end", i);
                let mut responses_lock = responses.lock().await;
                responses_lock.push(i);
            }));
        }

        for handle in handles {
            let _s = handle.await; // maybe consider handling errors propagated from the thread here
        }

        let responses_lock = responses.lock().await;
        assert_eq!(100, responses_lock.len());

        assert!(maybe.data.read().is_none());
    }

    #[tokio::test]
    async fn async_should_execute_serially() {
        let maybe = MaybeSingleAsync::new(|| Box::pin(async {}));
        let maybe = Arc::new(maybe);

        let responses = Arc::new(AsyncMutex::new(vec![]));

        let mut handles = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            let responses = responses.clone();
            let sleep_for = thread_rng().gen_range(0..10);

            handles.push(tokio::spawn(async move {
                let _data = maybe.data(true).await;
                assert!(maybe.data.read().is_some());
                println!(" exec {} start", i);
                tokio::time::sleep(Duration::from_nanos(sleep_for)).await;
                println!(" exec {} end", i);
                let mut responses_lock = responses.lock().await;
                responses_lock.push(i);
            }));
        }

        for handle in handles {
            let _ = handle.await; // maybe consider handling errors propagated from the thread here
        }

        let responses_lock = responses.lock().await;
        assert_eq!(100, responses_lock.len());

        assert!(maybe.data.read().is_none());
    }
}
