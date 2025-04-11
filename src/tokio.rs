use std::ops::Deref;
use std::sync::Arc;

use core::pin::Pin;
use log::info;
use parking_lot::{Mutex, RwLock};
use std::future::Future;
use tokio::sync::{RwLock as AsyncRwLock, RwLockReadGuard, RwLockWriteGuard};

/// A `MaybeOnce` object is a variation of the `OnceLock` object that keeps track of the number of references to the internal data
/// and drops it every time the references counter goes to 0. When the data is accessed, 
/// it will be created if it does not exist, or it will recreated if it was previously dropped.
/// 
/// This object is to be used to inizialize a shared resource that must be dropped when it is no longer used or when 
/// the process terminates. This mechanism is used as a workaround for the fact that rust does not drop static items at the end of the program.
/// 
/// A typical example is to initialize an expensive object, for example to start a docker container, to be used by a set of integration tests,
/// with the guarantee that it is properly dropped when the tests terminates.
/// 
/// Please note that, if this object is used in a single thread context, then it will drop the data after each access. 
/// This is caused by the fact that the internal reference counter will always be 0 once the data is dropped.
/// 
/// Example: 
/// ```rust
/// mod test {
/// 
///     use std::sync::OnceLock;
///     use maybe_once::tokio::{Data, MaybeOnceAsync};
/// 
/// 
/// /// A data initializer function. This can be called more than once.
/// /// If everything goes as expected, it should only be called once.
/// async fn init() -> String {
///     // Expensive initialization logic here.
///     // For example, you can start here a docker container (e.g. by using testcontainers), when there will be no more
///     // references to the data, the data will be dropped and the container will be stopped.
///     "hello".to_string()    
/// }
/// 
/// /// A function that returns a `Data` object.
/// pub async fn data(serial: bool) -> Data<'static, String> {
///     static DATA: OnceLock<MaybeOnceAsync<String>> = OnceLock::new();
///     DATA.get_or_init(|| MaybeOnceAsync::new(|| Box::pin(init())))
///         .data(serial)
///         .await
/// }
/// 
///     /// This test, and all the others, uses the data function to access the shared data.
///     /// The same data instance is shared between all the threads exactly like OnceLock does,
///     /// but when the all tests finish, the data will be dropped before the process terminates.
///     #[tokio::test]
///     async fn test1() {
///         let data = data(false).await;
///         println!("{}", *data);
///     }
/// 
///     #[tokio::test]
///     async fn test2() {
///         let data = data(false).await;
///         println!("{}", *data);
///     }
/// 
///     #[tokio::test]
///     async fn test3() {
///         let data = data(false).await;
///         println!("{}", *data);
///     }
/// 
/// } 
/// ```
pub struct MaybeOnceAsync<T> {
    data: Arc<RwLock<Option<Arc<T>>>>,
    lock_mutex: Arc<AsyncRwLock<()>>,
    init: fn() -> Pin<Box<dyn Send + Future<Output = T>>>,
    callers: Arc<Mutex<usize>>,
}

impl<T> MaybeOnceAsync<T> {

    /// Creates a new `MaybeOnceAsync` object with the given `init` function.
    ///
    /// `init` is a function that creates a new `T` object. It is lazily called the first time
    /// `data` is called and every time after the data is dropped.
    ///
    /// The returned `MaybeOnceAsync` object is then used to access the shared data with the
    /// `data` method.
    pub fn new(init: fn() -> Pin<Box<dyn Send + Future<Output = T>>>) -> Self {
        MaybeOnceAsync {
            data: Arc::new(RwLock::new(None)),
            init,
            lock_mutex: Arc::new(AsyncRwLock::new(())),
            callers: Arc::new(Mutex::new(0)),
        }
    }

    /// This function returns a `Data` object, which allows you to access the shared data.
    ///
    /// The `serial` parameter allows you to control whether the data is accessed in a serial
    /// or parallel manner. If `serial` is `true`, the data will be accessed in a serial manner,
    /// meaning that no other thread can access the data until the returned `Data` is dropped.
    /// If `serial` is `false`, the data will be accessed in a parallel manner, meaning that
    /// any number of threads can access the data at the same time.
    ///
    /// The returned `Data` object implements `Deref` and `AsRef`, so you can use it like a reference.
    ///
    /// The `Data` object also implements `Drop`, so when it goes out of scope, the lock is released.
    pub async fn data(&self, serial: bool) -> Data<'_, T> {
        {
            let mut lock = self.callers.lock();
            let callers = *lock + 1;
            *lock = callers;
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
                        *write_lock = Some(Arc::new(init));
                    }
                }
            };

            let lock = self.data.read();

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

/// A struct that allows you to access the shared data.
pub struct Data<'a, T> {
    data_arc: Arc<T>,
    data: Arc<RwLock<Option<Arc<T>>>>,
    #[allow(dead_code)]
    read_lock: Option<RwLockReadGuard<'a, ()>>,
    #[allow(dead_code)]
    write_lock: Option<RwLockWriteGuard<'a, ()>>,
    callers: Arc<Mutex<usize>>,
}

impl<T> Drop for Data<'_, T> {
    fn drop(&mut self) {
        let mut lock = self.callers.lock();
        // Here the lock cannot be less than 1
        let callers = *lock - 1;
        *lock = callers;

        if callers == 0 {
            info!("MaybeSingle --- Dropping DATA ---");
            let mut data = self.data.write();
            *data = None;
        }
    }
}

impl<T> Deref for Data<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data_arc.as_ref()
    }
}

impl<T> AsRef<T> for Data<'_, T> {
    fn as_ref(&self) -> &T {
        self.data_arc.as_ref()
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use rand::random_range;
    use std::time::Duration;
    use tokio::sync::Mutex as AsyncMutex;

    #[test]
    fn maybe_should_be_send() {
        let maybe = MaybeOnceAsync::new(|| Box::pin(async {}));
        need_send(maybe);
    }

    fn need_send<T: Send>(_t: T) {}
    fn need_sync<T: Sync>(_t: T) {}

    #[test]
    fn maybe_should_be_sync() {
        let maybe = MaybeOnceAsync::new(|| Box::pin(async {}));
        need_sync(maybe);
    }

    #[tokio::test]
    async fn async_should_execute_in_parallel() {
        let maybe = MaybeOnceAsync::new(|| Box::pin(async {}));
        let maybe = Arc::new(maybe);

        let responses = Arc::new(AsyncMutex::new(vec![]));

        let mut handles = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            let responses = responses.clone();
            let sleep_for = random_range(0..1000);
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
        let maybe = MaybeOnceAsync::new(|| Box::pin(async {}));
        let maybe = Arc::new(maybe);

        let responses = Arc::new(AsyncMutex::new(vec![]));

        let mut handles = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            let responses = responses.clone();
            let sleep_for = random_range(0..10);

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
