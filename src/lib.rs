use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;

pub struct MaybeSingle<T> {
    data: Arc<RwLock<Option<T>>>,
    init: fn() -> T,
    callers: Arc<Mutex<AtomicUsize>>
}

impl <T> MaybeSingle<T> {

    pub fn new(init: fn() -> T) -> Self {
        MaybeSingle {
            data: Arc::new(RwLock::new(None)),
            init,
            callers: Arc::new(Mutex::new(AtomicUsize::new(0)))
        }
    }

    pub fn get<F: FnOnce(&T)>(&self, callback: F) {

        //let rnd: u16 = rand::thread_rng().gen();

        //println!("---- Start {}", rnd);

        {
            let lock= self.callers.lock().unwrap();
            let callers = lock.load(SeqCst) + 1;
            lock.store(callers, SeqCst);
        }
        {
            let mut lock = self.data.write().unwrap();

            lock = if lock.is_none() {
                drop(lock);
                {
                    let mut write_lock = self.data.write().unwrap();
                    *write_lock = Some((self.init)());
                }
                self.data.write().unwrap()
            } else {
                lock
            };

            //println!("---- Exec {}", rnd);
            match lock.as_ref() {
                Some(data) => callback(data),
                None => panic!("There should always be data here!")
            };
        }
        {
            let lock= self.callers.lock().unwrap();
            let callers = lock.load(SeqCst) - 1;
            lock.store(callers, SeqCst);

            if callers == 0 {
                let mut data = self.data.write().unwrap();
                *data = None;
            }
        }
        //println!("---- End {}", rnd);

    }
}
