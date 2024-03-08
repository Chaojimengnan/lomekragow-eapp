use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Condvar, Mutex,
};

struct Inner<T> {
    pub container: Mutex<Vec<T>>,
    pub cond_var: Condvar,
    pub disconnect: AtomicBool,
}

#[derive(Debug)]
pub struct DisconnectError;

pub struct Sender<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Sender<T> {
    pub fn send(&self, t: T) -> Result<(), DisconnectError> {
        if self.inner.disconnect.load(Ordering::SeqCst) {
            return Err(DisconnectError);
        }

        self.inner.container.lock().unwrap().push(t);
        self.inner.cond_var.notify_one();

        Ok(())
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        self.inner.disconnect.store(true, Ordering::SeqCst);
    }
}

pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Receiver<T> {
    pub fn recv(&self) -> Result<T, DisconnectError> {
        if self.inner.disconnect.load(Ordering::SeqCst) {
            return Err(DisconnectError);
        }

        let mut container = self.inner.container.lock().unwrap();
        while container.is_empty() {
            container = self.inner.cond_var.wait(container).unwrap();
        }

        Ok(container.pop().unwrap())
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.inner.disconnect.store(true, Ordering::SeqCst);
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner {
        container: Mutex::new(Vec::with_capacity(128)),
        cond_var: Condvar::new(),
        disconnect: AtomicBool::new(false),
    });

    let sender = Sender {
        inner: inner.clone(),
    };

    (sender, Receiver { inner })
}
