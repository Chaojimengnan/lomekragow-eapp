use std::{
    sync::mpsc::Sender,
    thread::{JoinHandle, Result},
};

pub struct Task<T> {
    thread_handle: JoinHandle<T>,
    cancel_sender: Sender<()>,
}

impl<T> Task<T> {
    pub fn new<F>(cancel_sender: Sender<()>, f: F) -> Self
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        Self {
            thread_handle: std::thread::spawn(f),
            cancel_sender,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.thread_handle.is_finished()
    }

    pub fn cancel(&self) {
        self.cancel_sender.send(()).unwrap();
    }

    pub fn get_result(self) -> Result<T> {
        self.thread_handle.join()
    }
}
