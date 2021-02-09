use std::ops::Deref;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc};

use parking_lot::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
//use tokio::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::future::Future;
use core::pin::Pin;
use crate::Data;

pub struct MaybeSingleAsync<T> {
    data: Arc<RwLock<Option<Arc<T>>>>,
    lock_mutex: Arc<RwLock<()>>,
    init: fn() -> Pin<Box<dyn Future<Output = T>>>,
    callers: Arc<Mutex<AtomicUsize>>,
}

impl<T> MaybeSingleAsync<T> {

    pub fn new(init: fn() -> Pin<Box<dyn Future<Output = T>>>) -> Self {
        MaybeSingleAsync {
            data: Arc::new(RwLock::new(None)),
            init,
            lock_mutex: Arc::new(RwLock::new(())),
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
            let mut lock = self.data.read();

            lock = if lock.is_none() {
                drop(lock);
                {
                    let mut write_lock = self.data.write();

                    if write_lock.is_none() {
                        //  println!("--- INIT ---");
                        let init = {
                            (self.init)().await
                        };

                        *write_lock = Some(Arc::new(init));
                    }

                }
                self.data.read()
            } else {
                lock
            };
            //println!("---- Exec {}", rnd);
            match lock.as_ref() {
                Some(data) => data.clone(),
                None => panic!("There should always be data here!"),
            }
        };

        let (read_lock, write_lock) = if serial {
            (None, Some(self.lock_mutex.write()))
        } else {
            (Some(self.lock_mutex.read()), None)
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

#[cfg(test)]
mod test {

    use super::*;
    use rand::{thread_rng, Rng};
    use std::thread::sleep;
    use std::time::Duration;
    use async_std::sync::Mutex;

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


    #[async_std::test]
    async fn async_should_execute_in_parallel() {
        let maybe = MaybeSingleAsync::new(|| Box::pin(async {
        }));
        let maybe = Arc::new(maybe);

        let responses = Arc::new(Mutex::new(vec![]));

        let mut handles = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            let responses = responses.clone();
            handles.push(async_std::task::spawn_local( async move {
                let _data = maybe.data(false).await;
                println!(" exec {} start", i);
                async_std::task::sleep(Duration::from_nanos(thread_rng().gen_range(0..1000))).await;
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
    }

    #[async_std::test]
    async fn async_should_execute_serially() {
        let maybe = MaybeSingleAsync::new(|| Box::pin(async {
        }));
        let maybe = Arc::new(maybe);

        let responses = Arc::new(Mutex::new(vec![]));

        let mut handles = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            let responses = responses.clone();
            handles.push(async_std::task::spawn_local( async move {
                let _data = maybe.data(true).await;
                println!(" exec {} start", i);
                async_std::task::sleep(Duration::from_nanos(thread_rng().gen_range(0..10))).await;
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
    }

}
