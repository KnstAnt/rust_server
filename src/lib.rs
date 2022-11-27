use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};

#[macro_use]
extern crate log;

struct Worker {
    pub id: usize,
    pub handler: Option<thread::JoinHandle<()>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    pub fn new(threads_count: usize, stack_size: usize) -> ThreadPool {
        assert!(threads_count > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(threads_count.try_into().unwrap());

        for i in 0..threads_count {
            let receiver: Arc<Mutex<mpsc::Receiver<Job>>> = Arc::clone(&receiver);

            let id = i;

            let builder = if stack_size > 0 {
                thread::Builder::new().stack_size(stack_size)
            } else {
                thread::Builder::new()
            };

            match builder.spawn(move || loop {
                match receiver.lock().unwrap().recv() {
                    Ok(job) => {
                        info!("Worker {id} got a job; executing.");
                        job();
                    },
                    Err(_) => {
                        info!("Worker {id} disconnected; shutting down.");
                        break;
                    },
                }
            }) {
                Ok(handler) => workers.push(Worker {
                    id,
                    handler: Some(handler),
                }),
                Err(error) => {
                    error!("{:?}", error);
                    break;
                }
            }
        }

        ThreadPool { workers, sender: Some(sender) }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            info!("Shutting down worker {}", worker.id);

            if let Some(handler) = worker.handler.take() {
                handler.join().unwrap();
            }
        }
    }
}
