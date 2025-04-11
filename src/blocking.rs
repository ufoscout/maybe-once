use log::info;
use parking_lot::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::ops::Deref;
use std::sync::Arc;

/// A `MaybeOnce` object is a variation of the `OnceLock` object that keeps track of the number of references to the internal data
/// and drops it every time the references counter goes to 0. When the data is accessed,
/// it will be created if it does not exist, or it will recreated if it was previously dropped.
///
/// This object is to be used to inizialize a shared resource that must be dropped when it is no longer used or when
/// the process terminates. This mechanism is used as a workaround for the fact that rust does not drop static items at the end of the program.
///
/// This object is to be used to inizialize a shared resource that must be dropped when it is no longer used or when
/// the process terminates.
/// A typical example is to initialize an expensive object, for example to start a docker container, to be used by a set of integration tests,
/// with the guarantee that it is properly dropped when the tests terminates.
///
/// Example:
/// ```rust
/// mod test {
///
///     use std::sync::OnceLock;
///     use maybe_once::blocking::{Data, MaybeOnce};
///
///
/// /// A data initializer function. This can be called more than once.
/// /// If everything goes as expected, it should only be called once.
/// fn init() -> String {
///     // Expensive initialization logic here.
///     // For example, you can start here a docker container (e.g. by using testcontainers), when there will be no more
///     // references to the data, the data will be dropped and the container will be stopped.
///     "hello".to_string()    
/// }
///
/// /// A function that returns a `Data` object.
/// pub fn data(serial: bool) -> Data<'static, String> {
///     static DATA: OnceLock<MaybeOnce<String>> = OnceLock::new();
///     DATA.get_or_init(|| MaybeOnce::new(|| init()))
///         .data(serial)
/// }
///
///     /// This test, and all the others, uses the data function to access the shared data.
///     /// The same data instance is shared between all the threads exactly like OnceLock does,
///     /// but when the all tests finish, the data will be dropped before the process terminates.
///     #[test]
///     fn test1() {
///         let data = data(false);
///         println!("{}", *data);
///     }
///
///     #[test]
///     fn test2() {
///         let data = data(false);
///         println!("{}", *data);
///     }
///
///     #[test]
///     fn test3() {
///         let data = data(false);
///         println!("{}", *data);
///     }
///
/// }
/// ```
pub struct MaybeOnce<T> {
    data: Arc<RwLock<Option<Arc<T>>>>,
    lock_mutex: Arc<RwLock<()>>,
    init: fn() -> T,
    callers: Arc<Mutex<usize>>,
}

impl<T> MaybeOnce<T> {
    /// Creates a new `MaybeOnce` object with the given `init` function.
    ///
    /// `init` is a function that creates a new `T` object. It is lazily called the first time
    /// `data` is called and every time after the data is dropped.
    ///
    /// The returned `MaybeOnce` object is then used to access the shared data with the
    /// `data` method.
    pub fn new(init: fn() -> T) -> Self {
        MaybeOnce {
            data: Arc::new(RwLock::new(None)),
            init,
            lock_mutex: Arc::new(RwLock::new(())),
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
    pub fn data(&self, serial: bool) -> Data<'_, T> {
        {
            let mut lock = self.callers.lock();
            let callers = *lock + 1;
            *lock = callers;
        }
        let data_arc = {
            let mut lock = self.data.read();

            lock = if lock.is_none() {
                drop(lock);
                {
                    let mut write_lock = self.data.write();
                    if write_lock.is_none() {
                        *write_lock = Some(Arc::new((self.init)()));
                    }
                }
                self.data.read()
            } else {
                lock
            };

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
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn should_execute_in_parallel() {
        let maybe: MaybeOnce<()> = MaybeOnce::new(|| {});
        let maybe = Arc::new(maybe);
        let mut handles = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            handles.push(std::thread::spawn(move || {
                let _data = maybe.data(false);
                assert!(maybe.data.read().is_some());
                println!(" exec {} start", i);
                sleep(Duration::from_nanos(random_range(0..1000)));
                println!(" exec {} end", i);
            }));
        }

        for handle in handles {
            handle.join().unwrap(); // maybe consider handling errors propagated from the thread here
        }

        assert!(maybe.data.read().is_none());
    }

    #[test]
    fn should_execute_serially() {
        let maybe: MaybeOnce<()> = MaybeOnce::new(|| {});
        let maybe = Arc::new(maybe);
        let mut handles = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            handles.push(std::thread::spawn(move || {
                let _data = maybe.data(true);
                assert!(maybe.data.read().is_some());
                println!(" exec {} start", i);
                sleep(Duration::from_nanos(random_range(0..1000)));
                println!(" exec {} end", i);
            }));
        }

        for handle in handles {
            handle.join().unwrap(); // maybe consider handling errors propagated from the thread here
        }

        assert!(maybe.data.read().is_none());
    }
}
