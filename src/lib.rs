use std::ops::Deref;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

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

    pub fn get<F: FnOnce(&T)>(&self, no_parallel: bool, callback: F) {
        //let rnd: u16 = rand::thread_rng().gen();

        //println!("---- Start {}", rnd);

        {
            let lock = self.callers.lock().unwrap();
            let callers = lock.load(SeqCst) + 1;
            lock.store(callers, SeqCst);
        }
        let data_arc = {
            let mut lock = self.data.read().unwrap();

            lock = if lock.is_none() {
                drop(lock);
                {
                    let mut write_lock = self.data.write().unwrap();
                    if write_lock.is_none() {
                        //  println!("--- INIT ---");
                        *write_lock = Some(Arc::new((self.init)()));
                    }
                }
                self.data.read().unwrap()
            } else {
                lock
            };

            //println!("---- Exec {}", rnd);
            match lock.as_ref() {
                Some(data) => data.clone(),
                None => panic!("There should always be data here!"),
            }
        };

        let (read_lock, write_lock) = if no_parallel {
            (
                Arc::new(None),
                Arc::new(Some(self.lock_mutex.write().unwrap())),
            )
        } else {
            (
                Arc::new(Some(self.lock_mutex.read().unwrap())),
                Arc::new(None),
            )
        };

        let data_wrap = Data {
            data_arc,
            data: self.data.clone(),
            callers: self.callers.clone(),
            read_lock,
            write_lock,
        };
        callback(&data_wrap);
        {
            /*
            let lock= self.callers.lock().unwrap();
            let callers = lock.load(SeqCst) - 1;
            lock.store(callers, SeqCst);

            if callers == 0 {
                let mut data = self.data.write().unwrap();
                *data = None;
            }
            */
        }
        //println!("---- End {}", rnd);
    }
}

pub struct Data<'a, T> {
    data_arc: Arc<T>,
    data: Arc<RwLock<Option<Arc<T>>>>,
    read_lock: Arc<Option<RwLockReadGuard<'a, ()>>>,
    write_lock: Arc<Option<RwLockWriteGuard<'a, ()>>>,
    callers: Arc<Mutex<AtomicUsize>>,
}

impl<'a, T> Drop for Data<'a, T> {
    fn drop(&mut self) {
        //println!("--- Dropping DATA ---");
        let lock = self.callers.lock().unwrap();
        let callers = lock.load(SeqCst) - 1;
        lock.store(callers, SeqCst);

        if callers == 0 {
            let mut data = self.data.write().unwrap();
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
                maybe.get(false, |_| {
                    println!(" exec {} start", i);
                    sleep(Duration::from_nanos(thread_rng().gen_range(0, 1000)));
                    println!(" exec {} end", i);
                })
            }));
        }

        for handle in handles {
            let _ = handle.join().unwrap(); // maybe consider handling errors propagated from the thread here
        }
    }

    #[test]
    fn should_execute_serially() {
        let maybe: MaybeSingle<()> = MaybeSingle::new(|| {});
        let maybe = Arc::new(maybe);
        let mut handles = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            handles.push(std::thread::spawn(move || {
                maybe.get(true, |_| {
                    println!(" exec {} start", i);
                    sleep(Duration::from_nanos(thread_rng().gen_range(0, 1000)));
                    println!(" exec {} end", i);
                })
            }));
        }

        for handle in handles {
            let _ = handle.join().unwrap(); // maybe consider handling errors propagated from the thread here
        }
    }
}
