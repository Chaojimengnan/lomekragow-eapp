use eframe::egui;
use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    sync::mpsc::{RecvTimeoutError, Sender},
    time::{Duration, Instant},
};

pub struct Waker {
    sender: Sender<f64>,
}

pub enum WakeType {
    Independent,
    WakeOnLongestDeadLine,
}

impl Waker {
    pub fn new(ctx: egui::Context, wake_type: WakeType) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();

        match wake_type {
            WakeType::Independent => std::thread::spawn(move || {
                let mut heap = BinaryHeap::<Reverse<Instant>>::new();

                loop {
                    while let Ok(secs) = receiver.try_recv() {
                        let deadline = Instant::now() + Duration::from_secs_f64(secs);
                        heap.push(Reverse(deadline));
                    }

                    let now = Instant::now();
                    let mut need_repaint = false;

                    while let Some(Reverse(deadline)) = heap.peek() {
                        if *deadline <= now {
                            heap.pop();
                            need_repaint = true;
                        } else {
                            break;
                        }
                    }

                    if need_repaint {
                        ctx.request_repaint();
                    }

                    let sleep_duration = heap
                        .peek()
                        .map(|Reverse(deadline)| deadline.saturating_duration_since(now));

                    match sleep_duration {
                        Some(duration) => match receiver.recv_timeout(duration) {
                            Ok(secs) => {
                                let deadline = Instant::now() + Duration::from_secs_f64(secs);
                                heap.push(Reverse(deadline));
                            }
                            Err(RecvTimeoutError::Disconnected) => break,
                            Err(RecvTimeoutError::Timeout) => {}
                        },
                        None => match receiver.recv() {
                            Ok(secs) => {
                                let deadline = Instant::now() + Duration::from_secs_f64(secs);
                                heap.push(Reverse(deadline));
                            }
                            Err(_) => break,
                        },
                    }
                }
            }),
            WakeType::WakeOnLongestDeadLine => std::thread::spawn(move || {
                let mut longest_deadline: Option<Instant> = None;

                loop {
                    while let Ok(secs) = receiver.try_recv() {
                        let deadline = Instant::now() + Duration::from_secs_f64(secs);
                        if longest_deadline.is_none_or(|d| deadline > d) {
                            longest_deadline = Some(deadline);
                        }
                    }

                    let sleep_duration = longest_deadline
                        .map(|d| d.saturating_duration_since(Instant::now()))
                        .unwrap_or(Duration::from_secs(3600));

                    match receiver.recv_timeout(sleep_duration) {
                        Ok(secs) => {
                            let deadline = Instant::now() + Duration::from_secs_f64(secs);
                            if longest_deadline.is_none_or(|d| deadline > d) {
                                longest_deadline = Some(deadline);
                            }
                        }
                        Err(RecvTimeoutError::Disconnected) => break,
                        Err(RecvTimeoutError::Timeout) => {
                            if longest_deadline.is_some() {
                                ctx.request_repaint();
                                longest_deadline = None;
                            }
                        }
                    }
                }
            }),
        };

        Self { sender }
    }

    pub fn request_repaint_after_secs(&self, secs: f64) {
        let _ = self.sender.send(secs);
    }
}
