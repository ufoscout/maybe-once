use std::ops::Deref;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc};
use parking_lot::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
//use tokio::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::future::Future;
use tokio::macros::support::Pin;

pub struct MaybeSingle<T> {
    data: Arc<RwLock<Option<Arc<T>>>>,
    lock_mutex: Arc<RwLock<()>>,
    init: fn() -> Pin<Box<dyn Future<Output = T> + Send + Sync>>,
    callers: Arc<Mutex<AtomicUsize>>,
}

impl<T> MaybeSingle<T> {

    pub fn new(init: fn() -> Pin<Box<dyn Future<Output = T> + Send + Sync>>) -> Self {
        MaybeSingle {
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
                    let init = {
                      //  (self.init)().await
                    };
                    if write_lock.is_none() {
                        //  println!("--- INIT ---");
                        //*write_lock = Some(Arc::new(init));
                        *write_lock = None;
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
        async_drop(self)
    }
}

fn async_drop<'a, T>(target: &mut Data<'a, T>) {
    let lock = target.callers.lock();
    let callers = lock.load(SeqCst) - 1;
    lock.store(callers, SeqCst);

    if callers == 0 {
        let mut data = target.data.write();
       *data = None;
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
        let maybe = MaybeSingle::new(|| Box::pin(async {}));
        let maybe = Arc::new(maybe);
        let mut handles = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            handles.push(tokio::spawn(async move {
                let _data = maybe.data(false).await;
                println!(" exec {} start", i);
                sleep(Duration::from_nanos(thread_rng().gen_range(0, 1000)));
                println!(" exec {} end", i);
            }));
        }

        for handle in handles {
//            let _ = handle.join().unwrap(); // maybe consider handling errors propagated from the thread here
        }
    }
    /*
    #[test]
    fn should_execute_in_parallel() {
        let maybe = MaybeSingle::new(|| async {()});
        let maybe = Arc::new(maybe);
        let mut tasks = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            tasks.push(tokio::spawn(async move {
                let data = maybe.data(false).await;
                println!(" exec {} start", i);
                sleep(Duration::from_nanos(thread_rng().gen_range(100, 1000)));
                println!(" exec {} end", i);
                ()
            }));
        }

        println!("1");

        let mut handles = vec![];
        for task in tasks {
            handles.push(std::thread::spawn(move || {await_it(task)}));
        }

        println!("2");

        for handle in handles {
            let _ = handle.join().unwrap(); // maybe consider handling errors propagated from the thread here
        }

        println!("3");
    }

    #[tokio::test]
    async fn should_execute_serially() {
        let maybe = MaybeSingle::new(|| async {});
        let maybe = Arc::new(maybe);
        let mut tasks = vec![];

        for i in 0..100 {
            let maybe = maybe.clone();
            tasks.push(tokio::spawn(async move {
                let _data = maybe.data(true).await;
                println!(" exec {} start", i);
                sleep(Duration::from_nanos(thread_rng().gen_range(0, 1000)));
                println!(" exec {} end", i);
                ()
            }));
        }

        //let mut handles = vec![];
        for task in tasks {
            task.await;
            //handles.push(std::thread::spawn(move || {await_it(task)}));
        }

        //for handle in handles {
        //    let _ = handle.join().unwrap(); // maybe consider handling errors propagated from the thread here
        //}
    }

    #[tokio::main]
    async fn await_it<F: Future>(f: F) -> F::Output {
        f.await
    }
    */
}
