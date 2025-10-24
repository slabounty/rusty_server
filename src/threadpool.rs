use std::sync::{mpsc, Arc, Mutex};
use std::thread;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>, // Wrap in Option for safe drop
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    /// Create a new ThreadPool with the given number of threads.
    ///
    /// # Panics
    /// Panics if `size` is 0.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    /// Submit a job to be executed by the pool.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if let Some(sender) = &self.sender {
            sender.send(Box::new(f)).unwrap();
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Take the sender out of the Option to close the channel
        self.sender.take(); // Dropped here => channel closed

        // Join all threads
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    #[allow(dead_code)]
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();
            match message {
                Ok(job) => job(),
                Err(_) => break, // channel closed => exit thread
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

#[test]
fn test_threadpool_executes_jobs() {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    let pool = ThreadPool::new(4);
    let counter = Arc::new(Mutex::new(0));

    for _ in 0..10 {
        let c = Arc::clone(&counter);
        pool.execute(move || {
            let mut val = c.lock().unwrap();
            *val += 1;
        });
    }

    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(*counter.lock().unwrap(), 10);
}
