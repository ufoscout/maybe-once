use parking_lot::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::ops::Deref;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;

pub struct MaybeSingle<T> {
    data: Arc<RwLock<Option<Arc<T>>>>,
    lock_mutex: Arc<RwLock<()>>,
    init: fn() -> T,
    callers: Arc<Mutex<AtomicUsize>>,
}

impl<T> MaybeSingle<T> {
    pub fn new(init: fn() -> T) -> Self {
        MaybeSingle {
            data: Arc::new(RwLock::new(None)),
            init,
            lock_mutex: Arc::new(RwLock::new(())),
            callers: Arc::new(Mutex::new(AtomicUsize::new(0))),
        }
    }

    pub fn data(&self, serial: bool) -> Data<'_, T> {
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
                        *write_lock = Some(Arc::new((self.init)()));
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

pub struct Data<'a, T> {
    data_arc: Arc<T>,
    data: Arc<RwLock<Option<Arc<T>>>>,
    read_lock: Option<RwLockReadGuard<'a, ()>>,
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
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn should_execute_in_parallel() {
        let maybe: MaybeSingle<()> = MaybeSingle::new(|| {});
        let maybe = Arc::new(maybe);
        let mut handles = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            handles.push(std::thread::spawn(move || {
                let _data = maybe.data(false);
                assert!(maybe.data.read().is_some());
                println!(" exec {} start", i);
                sleep(Duration::from_nanos(thread_rng().gen_range(0..1000)));
                println!(" exec {} end", i);
            }));
        }

        for handle in handles {
            let _ = handle.join().unwrap(); // maybe consider handling errors propagated from the thread here
        }

        assert!(maybe.data.read().is_none());
    }

    #[test]
    fn should_execute_serially() {
        let maybe: MaybeSingle<()> = MaybeSingle::new(|| {});
        let maybe = Arc::new(maybe);
        let mut handles = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            handles.push(std::thread::spawn(move || {
                let _data = maybe.data(true);
                assert!(maybe.data.read().is_some());
                println!(" exec {} start", i);
                sleep(Duration::from_nanos(thread_rng().gen_range(0..1000)));
                println!(" exec {} end", i);
            }));
        }

        for handle in handles {
            let _ = handle.join().unwrap(); // maybe consider handling errors propagated from the thread here
        }

        assert!(maybe.data.read().is_none());
    }
}
