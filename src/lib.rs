use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;

pub struct MaybeSingle<T> {
    data: Arc<Mutex<Option<T>>>,
    init: fn() -> T,
    callers: Arc<Mutex<AtomicUsize>>
}

impl <T> MaybeSingle<T> {

    pub fn new(init: fn() -> T) -> Self {
        MaybeSingle {
            data: Arc::new(Mutex::new(None)),
            init,
            callers: Arc::new(Mutex::new(AtomicUsize::new(0)))
        }
    }

    pub fn get<F: FnOnce(&T)>(&self, callback: F) {
        {
            let lock= self.callers.lock().unwrap();
            let callers = lock.load(SeqCst) + 1;
            lock.store(callers, SeqCst);
        }
        {
            let mut lock = self.data.lock().unwrap();

            if lock.is_none() {
                *lock = Some((self.init)());
            }

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
                let mut data = self.data.lock().unwrap();
                *data = None;
            }
        }

    }
}
