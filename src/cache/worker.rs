use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
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
    current: Arc<AtomicUsize>,
    workers: Vec<JoinHandle<()>>,
}

impl Loader {
    pub fn new(threads: usize) -> Self {
        let shared = Arc::new((Mutex::new(Queue::default()), Condvar::new()));
        let (tx, results) = crossbeam_channel::unbounded();
        let threads = threads.max(1);
        let current = Arc::new(AtomicUsize::new(0));

        let workers = (0..threads)
            .map(|_| {
                let shared = Arc::clone(&shared);
                let current = Arc::clone(&current);
                let tx = tx.clone();
                std::thread::spawn(move || worker_loop(shared, current, tx))
            })
            .collect();

        Self { shared, results, next_id: AtomicU64::new(1), current, workers }
    }

    pub fn set_current_index(&self, index: usize) {
        self.current.store(index, Ordering::Relaxed);
    }

    pub fn cancel_far_from(&self, index: usize, radius: usize) -> Vec<RequestId> {
        let (lock, _) = &*self.shared;
        let mut queue = lock.lock().unwrap();
        let dropped: Vec<RequestId> = queue
            .pending
            .iter()
            .filter(|r| r.index.abs_diff(index) > radius)
            .map(|r| r.id)
            .collect();
        queue.pending.retain(|r| r.index.abs_diff(index) <= radius);
        queue.cancelled.extend(dropped.iter().copied());
        dropped
    }

    pub fn cancel_everything(&self) -> Vec<RequestId> {
        let (lock, _) = &*self.shared;
        let mut queue = lock.lock().unwrap();
        let dropped: Vec<RequestId> = queue.pending.iter().map(|r| r.id).collect();
        queue.pending.clear();
        queue.cancelled.extend(dropped.iter().copied());
        dropped
    }

    pub fn request(
        &self,
        path: PathBuf,
        index: usize,
        target: (u32, u32),
        resample: crate::render::Resample,
    ) -> RequestId {
        let id = RequestId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let (lock, cvar) = &*self.shared;
        let mut queue = lock.lock().unwrap();
        queue.pending.push(LoadRequest {
            id,
            path,
            index,
            target_width: target.0,
            target_height: target.1,
            resample,
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

fn worker_loop(
    shared: Arc<(Mutex<Queue>, Condvar)>,
    current: Arc<AtomicUsize>,
    tx: crossbeam_channel::Sender<LoadResult>,
) {
    loop {
        let request = {
            let (lock, cvar) = &*shared;
            let mut queue = lock.lock().unwrap();
            loop {
                if queue.shutdown {
                    return;
                }
                if let Some(request) = take_next(&mut queue, current.load(Ordering::Relaxed)) {
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

fn take_next(queue: &mut Queue, current: usize) -> Option<LoadRequest> {
    loop {
        let best = queue
            .pending
            .iter()
            .enumerate()
            .min_by_key(|(_, r)| (r.index.abs_diff(current), std::cmp::Reverse(r.id.0)))
            .map(|(i, _)| i)?;
        let request = queue.pending.remove(best);
        if !queue.cancelled.remove(&request.id) {
            return Some(request);
        }
    }
}

fn load(request: &LoadRequest) -> Result<super::CachedImage, DecodeError> {
    let bytes = std::fs::read(&request.path)?;
    let output = crate::panic_report::suppress_during(|| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            decode::decode(&DecodeRequest {
                path: &request.path,
                bytes: &bytes,
                target_width: request.target_width,
                resample: request.resample,
                target_height: request.target_height,
            })
        }))
    })
    .map_err(|_| DecodeError::Decode("decoder panicked on this file".to_owned()))??;
    Ok(super::CachedImage {
        path: request.path.clone(),
        bytes: measure(&output),
        output: Arc::new(output),
    })
}
