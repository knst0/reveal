use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

use super::{LoadRequest, RequestId, measure};
use crate::decode::{self, DecodeError, DecodeRequest};

#[derive(Debug)]
pub struct LoadResult {
    pub id: RequestId,
    pub index: usize,
    pub path: PathBuf,
    pub outcome: Result<super::CachedImage, DecodeError>,
}

#[derive(Default)]
struct Queue {
    pending: Vec<LoadRequest>,
    cancelled: HashSet<RequestId>,
    shutdown: bool,
}

pub struct Loader {
    shared: Arc<(Mutex<Queue>, Condvar)>,
    results: crossbeam_channel::Receiver<LoadResult>,
    next_id: AtomicU64,
    workers: Vec<JoinHandle<()>>,
}

impl Loader {
    pub fn new(threads: usize) -> Self {
        let shared = Arc::new((Mutex::new(Queue::default()), Condvar::new()));
        let (tx, results) = crossbeam_channel::unbounded();
        let threads = threads.max(1);

        let workers = (0..threads)
            .map(|_| {
                let shared = Arc::clone(&shared);
                let tx = tx.clone();
                std::thread::spawn(move || worker_loop(shared, tx))
            })
            .collect();

        Self { shared, results, next_id: AtomicU64::new(1), workers }
    }

    pub fn request(&self, path: PathBuf, index: usize, target: (u32, u32)) -> RequestId {
        let id = RequestId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let (lock, cvar) = &*self.shared;
        let mut queue = lock.lock().unwrap();
        queue.pending.push(LoadRequest {
            id,
            path,
            index,
            target_width: target.0,
            target_height: target.1,
        });
        cvar.notify_one();
        id
    }

    pub fn cancel(&self, id: RequestId) {
        let (lock, _) = &*self.shared;
        let mut queue = lock.lock().unwrap();
        queue.pending.retain(|r| r.id != id);
        queue.cancelled.insert(id);
    }

    pub fn cancel_all_except(&self, keep: &[RequestId]) {
        let (lock, _) = &*self.shared;
        let mut queue = lock.lock().unwrap();
        let dropped: Vec<RequestId> =
            queue.pending.iter().filter(|r| !keep.contains(&r.id)).map(|r| r.id).collect();
        queue.pending.retain(|r| keep.contains(&r.id));
        queue.cancelled.extend(dropped);
    }

    pub fn pending_len(&self) -> usize {
        self.shared.0.lock().unwrap().pending.len()
    }

    pub fn try_recv(&self) -> Option<LoadResult> {
        self.results.try_recv().ok()
    }

    pub fn recv(&self) -> Option<LoadResult> {
        self.results.recv().ok()
    }
}

impl Drop for Loader {
    fn drop(&mut self) {
        {
            let (lock, cvar) = &*self.shared;
            let mut queue = lock.lock().unwrap();
            queue.shutdown = true;
            queue.pending.clear();
            cvar.notify_all();
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn worker_loop(shared: Arc<(Mutex<Queue>, Condvar)>, tx: crossbeam_channel::Sender<LoadResult>) {
    loop {
        let request = {
            let (lock, cvar) = &*shared;
            let mut queue = lock.lock().unwrap();
            loop {
                if queue.shutdown {
                    return;
                }
                if let Some(request) = take_next(&mut queue) {
                    break request;
                }
                queue = cvar.wait(queue).unwrap();
            }
        };

        let outcome = load(&request);

        {
            let (lock, _) = &*shared;
            let mut queue = lock.lock().unwrap();
            if queue.cancelled.remove(&request.id) {
                continue;
            }
        }

        if tx
            .send(LoadResult { id: request.id, index: request.index, path: request.path, outcome })
            .is_err()
        {
            return;
        }
    }
}

fn take_next(queue: &mut Queue) -> Option<LoadRequest> {
    while let Some(request) = queue.pending.pop() {
        if !queue.cancelled.remove(&request.id) {
            return Some(request);
        }
    }
    None
}

fn load(request: &LoadRequest) -> Result<super::CachedImage, DecodeError> {
    let bytes = std::fs::read(&request.path)?;
    let output = decode::decode(&DecodeRequest {
        path: &request.path,
        bytes: &bytes,
        target_width: request.target_width,
        target_height: request.target_height,
    })?;
    Ok(super::CachedImage {
        path: request.path.clone(),
        bytes: measure(&output),
        output: Arc::new(output),
    })
}
