use crossbeam_channel;
use crossbeam_channel::{Receiver, Sender};

use std::process;
use std::thread;
use std::thread::JoinHandle;

pub struct ThreadPool {
    num_threads: usize,
    handles: Vec<JoinHandle<()>>,
    sender: Option<Sender<JobFunc>>,
}

struct Thread;

type JobFunc = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    pub fn new(num_threads: usize) -> Self {
        let (sender, receiver) = crossbeam_channel::bounded(num_threads);

        let mut handles = Vec::with_capacity(num_threads);
        for _ in 0..num_threads {
            let thread_receiver = receiver.clone();
            handles.push(thread::spawn(move || Thread::new().park(thread_receiver)))
        }

        ThreadPool {
            num_threads,
            handles,
            sender: Some(sender),
        }
    }

    pub fn execute<F: FnOnce() + Send + 'static>(&self, job: F) {
        self.sender
            .as_ref()
            .expect("Thread pool already joined")
            .send(Box::new(job))
            .expect("Unable to send job to threadpool")
    }

    pub fn join(&mut self) {
        self.sender = None;
        while let Some(handle) = self.handles.pop() {
            handle.join().expect("Worker thread panicked");
        }
    }

    pub fn num_threads(&self) -> usize {
        self.num_threads
    }
}

impl Thread {
    fn new() -> Self {
        Thread
    }

    fn park(&self, receiver: Receiver<JobFunc>) {
        while let Ok(job) = receiver.recv() {
            job();
        }
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        if thread::panicking() {
            println!("Pool thread panicked, exiting");
            process::exit(1);
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.join();
    }
}
