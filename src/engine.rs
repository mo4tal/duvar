use crate::{
    model::{Config, Wallpaper},
    providers::{self, Network},
    store::Pixels,
};
use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{Receiver, SyncSender, TrySendError, sync_channel},
    },
};
#[derive(Clone)]
pub struct Request {
    pub number: u64,
    pub cancel: Arc<AtomicBool>,
}
pub enum Event {
    Items(u64, Vec<Wallpaper>),
    SearchFinished(u64, String, bool, usize, Option<String>),
    ThumbnailFailed(u64, String),
    Thumbnail(u64, String, Pixels),
    Preview(u64, String, Pixels),
    PreviewFailed(u64, String, String),
    Library(crate::library::Index),
    Metadata(String, crate::runtime::Facts),
    Trashed(crate::files::TrashTicket),
    Restored(crate::model::DownloadedImage),
    Notice(String),
    Downloaded(u64, Wallpaper, PathBuf, Option<String>),
    Failure(u64, String),
    ActionFailure(u64, crate::errors::Failure),
    DownloadStarted(u64),
    DownloadStopped(u64),
    ConfigSaved,
    ConfigFailed(String),
    KeyLoaded(String),
    KeySaved,
    KeyRemoved,
    KeyFailure(String, String),
}
pub enum Job {
    #[cfg(test)]
    Panic,
    Search(Request, String, Config, u32, String),
    Thumbnail(Request, Wallpaper),
    Preview(Request, Wallpaper),
    Download(u64, Request, Wallpaper, PathBuf, Option<String>),
    Downloads(Config),
    Metadata(Request, Wallpaper),
    Trash(crate::model::DownloadedImage),
    Restore(crate::files::TrashTicket),
    SaveQueue(
        u64,
        Vec<crate::model::DownloadRecord>,
        crate::queue::ApplyOrder,
        bool,
    ),
    Cache(String),
    Export(PathBuf),
    SaveConfig(u64, Config),
    SaveKey(String, String),
    RemoveKey(String),
    LoadKey(String),
}
impl Job {
    fn request(&self) -> Option<&Request> {
        match self {
            Self::Search(r, ..)
            | Self::Thumbnail(r, ..)
            | Self::Preview(r, ..)
            | Self::Metadata(r, ..) => Some(r),
            Self::Download(_, r, ..) => Some(r),
            _ => None,
        }
    }
}
struct Inner {
    queue: Mutex<[VecDeque<Job>; 6]>,
    ready: Condvar,
    stop: AtomicBool,
    pub pending: AtomicUsize,
    sender: SyncSender<Event>,
    network: Network,
    progress: Mutex<BTreeMap<u64, crate::runtime::Progress>>,
}
pub struct Pool {
    inner: Arc<Inner>,
}
impl Pool {
    pub fn new() -> (Self, Receiver<Event>) {
        let (sender, receiver) = sync_channel(64);
        let inner = Arc::new(Inner {
            queue: Mutex::new(std::array::from_fn(|_| VecDeque::new())),
            ready: Condvar::new(),
            stop: AtomicBool::new(false),
            pending: AtomicUsize::new(0),
            sender,
            network: Network::new(),
            progress: Mutex::new(BTreeMap::new()),
        });
        // Lane 0: Search (4) · Lane 1: Thumbnails (8, LIFO viewport priority)
        // Lane 2: Downloads (2) · Lane 3: Config/Keys/Trash (1)
        // Lane 4: Library scans (1) · Lane 5: Previews & Metadata (2, isolated from thumbnail load)
        for (lane, count) in [(0, 4), (1, 8), (2, 2), (3, 1), (4, 1), (5, 2)] {
            for _ in 0..count {
                let inner = inner.clone();
                std::thread::spawn(move || worker(inner, lane));
            }
        }
        (Self { inner }, receiver)
    }
    pub fn push(&self, job: Job) -> bool {
        let mut queues = self.inner.queue.lock().unwrap_or_else(|e| e.into_inner());
        let lane = match &job {
            Job::Search(..) => 0,
            Job::Thumbnail(..) => 1,
            Job::Download(..) => 2,
            Job::Downloads(..) => 4,
            Job::Preview(..) | Job::Metadata(..) => 5,
            _ => 3,
        };
        let q = &mut queues[lane];
        let cap = match lane {
            1 => 256,
            2 => 200,
            _ => 96,
        };
        if self.inner.stop.load(Ordering::Relaxed) {
            return false;
        }
        // For thumbnails, keep the newest viewport items by evicting the oldest off-screen tail.
        if lane == 1 && q.len() >= cap {
            if q.pop_back().is_some() {
                self.inner.pending.fetch_sub(1, Ordering::Relaxed);
            }
        } else if q.len() >= cap {
            return false;
        }
        if matches!(
            job,
            Job::SaveConfig(..) | Job::SaveQueue(..) | Job::Downloads(..)
        ) {
            let old = q.len();
            q.retain(|j| std::mem::discriminant(j) != std::mem::discriminant(&job));
            self.inner
                .pending
                .fetch_sub(old - q.len(), Ordering::Relaxed);
        }
        self.inner.pending.fetch_add(1, Ordering::Relaxed);
        if matches!(
            job,
            Job::Thumbnail(..) | Job::Preview(..) | Job::SaveConfig(..)
        ) {
            q.push_front(job);
        } else {
            q.push_back(job);
        }
        self.inner.ready.notify_all();
        true
    }
    pub fn drop_thumbnail(&self, id: &str) {
        let mut queues = self.inner.queue.lock().unwrap_or_else(|e| e.into_inner());
        let q = &mut queues[1];
        let old = q.len();
        q.retain(|j| !matches!(j, Job::Thumbnail(_, item) if item.id == id));
        let removed = old - q.len();
        if removed > 0 {
            self.inner.pending.fetch_sub(removed, Ordering::Relaxed);
        }
    }
    pub fn cancel_queued(&self) {
        let mut queues = self.inner.queue.lock().unwrap_or_else(|e| e.into_inner());
        for q in queues.iter_mut() {
            let old = q.len();
            q.retain(|j| {
                matches!(j, Job::Download(..))
                    || j.request()
                        .is_none_or(|r| !r.cancel.load(Ordering::Relaxed))
            });
            self.inner
                .pending
                .fetch_sub(old - q.len(), Ordering::Relaxed);
        }
    }
    pub fn progress(&self) -> Vec<crate::runtime::Progress> {
        std::mem::take(
            &mut *self
                .inner
                .progress
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
        )
        .into_values()
        .collect()
    }
    pub fn prioritize(&self, id: u64) {
        let mut queues = self.inner.queue.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(i) = queues[2]
            .iter()
            .position(|j| matches!(j, Job::Download(n, ..) if *n==id))
            && let Some(job) = queues[2].remove(i)
        {
            queues[2].push_front(job);
        }
    }
    pub fn pending(&self) -> usize {
        self.inner.pending.load(Ordering::Acquire)
    }
    pub fn close(&self) {
        self.inner.stop.store(true, Ordering::Relaxed);
        let mut q = self.inner.queue.lock().unwrap_or_else(|e| e.into_inner());
        let n: usize = q.iter().map(VecDeque::len).sum();
        for lane in q.iter_mut() {
            lane.clear();
        }
        self.inner.pending.fetch_sub(n, Ordering::Relaxed);
        self.inner.ready.notify_all();
    }
}
impl Inner {
    fn send(&self, mut event: Event) -> Result<(), ()> {
        loop {
            if self.stop.load(Ordering::Relaxed) {
                return Err(());
            }
            match self.sender.try_send(event) {
                Ok(()) => return Ok(()),
                Err(TrySendError::Disconnected(_)) => return Err(()),
                Err(TrySendError::Full(e)) => {
                    event = e;
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        }
    }
}
fn worker(inner: Arc<Inner>, lane: usize) {
    unsafe {
        libc::setpriority(
            libc::PRIO_PROCESS,
            libc::syscall(libc::SYS_gettid) as u32,
            10,
        );
    }
    loop {
        let job = {
            let mut q = inner.queue.lock().unwrap_or_else(|e| e.into_inner());
            while q[lane].is_empty() && !inner.stop.load(Ordering::Relaxed) {
                q = inner.ready.wait(q).unwrap_or_else(|e| e.into_inner());
            }
            if inner.stop.load(Ordering::Relaxed) {
                return;
            }
            q[lane].pop_front().unwrap()
        };
        if job
            .request()
            .is_some_and(|r| r.cancel.load(Ordering::Relaxed))
        {
            if let Job::Download(id, ..) = job {
                let _ = inner.send(Event::DownloadStopped(id));
            }
            inner.pending.fetch_sub(1, Ordering::Release);
            continue;
        }
        let action = if let Job::Download(id, ..) = &job {
            Some(*id)
        } else {
            None
        };
        let number = job.request().map(|r| r.number).unwrap_or(0);
        let cancelled = job.request().map(|r| r.cancel.clone());
        let _span = crate::diagnostics::Span::new(action.or(Some(number)), "job");
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> anyhow::Result<()> {
                match job {
                    #[cfg(test)]
                    Job::Panic => panic!("deliberate isolated worker fault"),
                    Job::Search(request, provider, config, page, key) => {
                        let mut emit = |item| {
                            if !request.cancel.load(Ordering::Relaxed) {
                                let _ = inner.send(Event::Items(number, vec![item]));
                            }
                        };
                        let result = inner.network.search(
                            &provider,
                            &config,
                            page,
                            &key,
                            &request.cancel,
                            if provider == "Wallhaven" {
                                Some(&mut emit)
                            } else {
                                None
                            },
                        );
                        if !request.cancel.load(Ordering::Relaxed) {
                            let (more, rejected, error) = match result {
                                Ok(batch) => {
                                    if provider != "Wallhaven" {
                                        let _ = inner.send(Event::Items(number, batch.items));
                                    }
                                    (batch.has_more, batch.rejected, None)
                                }
                                Err(e) => (true, 0, Some(format!("{provider}: {e}"))),
                            };
                            let _ = inner.send(Event::SearchFinished(
                                number, provider, more, rejected, error,
                            ));
                        }
                    }
                    Job::Thumbnail(r, item) => {
                        let pixels = match inner.network.thumbnail(&item, &r.cancel) {
                            Ok(p) => p,
                            Err(_) => {
                                let _ = inner.send(Event::ThumbnailFailed(number, item.id));
                                return Ok(());
                            }
                        };
                        if !r.cancel.load(Ordering::Relaxed) {
                            let _ = inner.send(Event::Thumbnail(number, item.id, pixels));
                        }
                    }
                    Job::Preview(r, item) => {
                        let pixels = match inner.network.preview(&item, &r.cancel) {
                            Ok(p) => p,
                            Err(e) => {
                                if !r.cancel.load(Ordering::Relaxed) {
                                    let _ = inner.send(Event::PreviewFailed(
                                        number,
                                        item.id,
                                        e.to_string(),
                                    ));
                                }
                                return Ok(());
                            }
                        };
                        if !r.cancel.load(Ordering::Relaxed) {
                            let _ = inner.send(Event::Preview(number, item.id.clone(), pixels));
                            if let Ok(facts) = inner.network.metadata(&item, &r.cancel) {
                                let _ = inner.send(Event::Metadata(item.id, facts));
                            }
                        }
                    }
                    Job::Download(id, r, item, directory, scope) => {
                        let _ = inner.send(Event::DownloadStarted(id));
                        let image = item.id.clone();
                        let (path, facts) = inner.network.download_progress(
                            &item,
                            &directory,
                            &r.cancel,
                            &mut |phase, received, total, speed, eta| {
                                inner
                                    .progress
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner())
                                    .insert(
                                        id,
                                        crate::runtime::Progress {
                                            job: id,
                                            image: image.clone(),
                                            phase,
                                            received,
                                            total,
                                            bytes_per_second: speed,
                                            eta_seconds: eta,
                                            at: crate::runtime::now(),
                                        },
                                    );
                            },
                        )?;
                        let _ = inner.send(Event::Metadata(item.id.clone(), facts));
                        let _ = inner.send(Event::Downloaded(
                            id,
                            item,
                            path,
                            scope.filter(|_| !r.cancel.load(Ordering::Relaxed)),
                        ));
                    }
                    Job::SaveConfig(revision, config) => {
                        match crate::store::save_config_revision(revision, &config) {
                            Ok(()) => {
                                let _ = inner.send(Event::ConfigSaved);
                            }
                            Err(e) => {
                                let _ = inner.send(Event::ConfigFailed(e.to_string()));
                            }
                        }
                    }
                    Job::Downloads(config) => {
                        crate::store::migrate_sidecars(&config)?;
                        let _ = inner.send(Event::Library(crate::library::load()?));
                    }
                    Job::Metadata(r, item) => {
                        let facts = inner.network.metadata(&item, &r.cancel)?;
                        if !r.cancel.load(Ordering::Relaxed) {
                            let _ = inner.send(Event::Metadata(item.id, facts));
                        }
                    }
                    Job::Trash(record) => {
                        let _ = inner.send(Event::Trashed(crate::files::trash(&record)?));
                    }
                    Job::Restore(ticket) => {
                        let record = crate::files::restore(&ticket)?;
                        let _ = inner.send(Event::Restored(record));
                    }
                    Job::SaveQueue(revision, entries, latest, paused) => {
                        crate::store::save_queue_revision(revision, &entries, latest, paused)?
                    }
                    Job::Cache(part) => {
                        if part != "stats" {
                            crate::cache::clear(&part)?;
                        }
                        let stats = crate::cache::stats()?;
                        let _ = inner.send(Event::Notice(format!(
                            "Önbellek: {} küçük resim · {} özgün · {} · {} bozuk",
                            stats.thumbnails,
                            stats.originals,
                            crate::runtime::size(stats.bytes),
                            stats.corrupt
                        )));
                    }
                    Job::Export(path) => {
                        crate::diagnostics::export(&path)?;
                        let _ = inner.send(Event::Notice(
                            "Tanı raporu kaydedildi; görsel ve anahtar içermez.".into(),
                        ));
                    }
                    Job::SaveKey(provider, key) => {
                        match providers::save_key_for(&provider, &key, &inner.stop) {
                            Ok(()) => {
                                let _ = inner.send(Event::KeySaved);
                            }
                            Err(e) => {
                                let _ = inner.send(Event::KeyFailure(provider, e.to_string()));
                            }
                        }
                    }
                    Job::RemoveKey(provider) => {
                        crate::providers::remove_key_for(&provider, &inner.stop)?;
                        let _ = inner.send(Event::KeyRemoved);
                    }
                    Job::LoadKey(provider) => {
                        let key = providers::load_key_for(&provider, &inner.stop)?;
                        let _ = inner.send(Event::KeyLoaded(key));
                    }
                };
                Ok(())
            }))
            .unwrap_or_else(|_| {
                Err(crate::errors::Failure::new(
                    crate::errors::Code::Panic,
                    "İş güvenli biçimde durduruldu; tekrar denenebilir",
                )
                .into())
            });
        if let Err(e) = result {
            let error = e
                .downcast_ref::<crate::errors::Failure>()
                .cloned()
                .unwrap_or_else(|| crate::errors::Failure::classify(&e.to_string()));
            crate::diagnostics::failure(action.or(Some(number)), "job", &error);
            if !inner.stop.load(Ordering::Relaxed)
                && !cancelled
                    .as_ref()
                    .is_some_and(|c| c.load(Ordering::Relaxed))
            {
                let event = if let Some(id) = action {
                    Event::ActionFailure(id, error)
                } else {
                    Event::Failure(number, e.to_string())
                };
                let _ = inner.send(event);
            }
        }
        if let Some(id) = action {
            let _ = inner.send(Event::DownloadStopped(id));
        }
        inner.pending.fetch_sub(1, Ordering::Release);
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_receiver_cannot_hold_worker() {
        let (pool, rx) = Pool::new();
        drop(rx);
        assert!(pool.inner.send(Event::Notice("test".into())).is_err());
        pool.close();
        assert!(!pool.push(Job::Cache("stats".into())));
    }
    #[test]
    fn full_channel_is_interruptible() {
        let (pool, _rx) = Pool::new();
        for _ in 0..64 {
            pool.inner
                .sender
                .try_send(Event::Notice("test".into()))
                .unwrap();
        }
        let inner = pool.inner.clone();
        let handle = std::thread::spawn(move || inner.send(Event::Notice("overflow".into())));
        std::thread::sleep(std::time::Duration::from_millis(15));
        pool.close();
        assert!(handle.join().unwrap().is_err());
    }
    #[test]
    fn a_panicking_job_does_not_remove_a_worker() {
        let (pool, rx) = Pool::new();
        assert!(pool.push(Job::Panic));
        assert!(matches!(
            rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap(),
            Event::Failure(..)
        ));
        assert!(pool.push(Job::Cache("stats".into())));
        assert!(matches!(
            rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap(),
            Event::Notice(..)
        ));
        pool.close();
    }
}
