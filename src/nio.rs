use std::ops::Deref;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc};

use parking_lot::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
//use tokio::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::future::Future;
use core::pin::Pin;
pub use futures::FutureExt;
pub use crate::Data;

pub struct MaybeSingleAsync<T: 'static> {
    data: Arc<RwLock<Option<Arc<T>>>>,
    lock_mutex: Arc<RwLock<()>>,
    init: fn() -> Pin<Box<dyn Future<Output = T>>>,
    callers: Arc<Mutex<AtomicUsize>>,
}

impl<T: 'static + Send> MaybeSingleAsync<T> {

    pub fn new(init: fn() -> Pin<Box<dyn Future<Output = T>>>) -> Self {
        MaybeSingleAsync {
            data: Arc::new(RwLock::new(None)),
            init,
            lock_mutex: Arc::new(RwLock::new(())),
            callers: Arc::new(Mutex::new(AtomicUsize::new(0))),
        }
    }

    pub async fn data<'a>(&'a self, serial: bool) -> Data<'a, T> {
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
    use tokio::time::Instant;

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

    /*
    #[tokio::test]
    async fn should_execute_in_parallel() {
        let maybe = MaybeSingleAsync::new(|| Box::pin(async {
        }));
        let maybe = Arc::new(maybe);
        let _data = maybe.data(false).await;

        let maybe_clone = maybe.clone();
        tokio::spawn(async move {
        //    let _data = maybe_clone.data(false).await;
        });

        let mut handles = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            handles.push(tokio::spawn( async move {
                let _data = maybe.data(true).await;
                println!(" exec {} start", i);
                tokio::time::delay_until(Instant::now() + Duration::from_nanos(thread_rng().gen_range(0, 1000))).await;
                println!(" exec {} end", i);
            }));

        }

        for handle in handles {
            let _ = handle.await.unwrap(); // maybe consider handling errors propagated from the thread here
        }
    }

    #[tokio::test]
    async fn should_execute_serially() {
        let maybe = MaybeSingleAsync::new(|| Box::pin(async {
        }));
        let maybe = Arc::new(maybe);
        let _data = maybe.data(false).await;

        let maybe_clone = maybe.clone();
        tokio::spawn(async move {
            //    let _data = maybe_clone.data(false).await;
        });

        let mut handles = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            handles.push(tokio::spawn( async move {
                let _data = maybe.data(true).await;
                println!(" exec {} start", i);
                tokio::time::delay_until(Instant::now() + Duration::from_nanos(thread_rng().gen_range(0, 1000))).await;
                println!(" exec {} end", i);
            }));

        }

        for handle in handles {
            let _ = handle.await.unwrap(); // maybe consider handling errors propagated from the thread here
        }
    }
    */

}
