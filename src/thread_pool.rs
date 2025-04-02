use std::{
    marker::PhantomData,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc,
        atomic::AtomicUsize,
        mpsc::{self, Sender},
    },
};

pub struct ThreadPool {
    threads: Box<[ThreadHandle]>,
}

impl ThreadPool {
    pub fn new(thread_count: usize) -> Self {
        let mut threads = Vec::with_capacity(thread_count);
        for id in 0..thread_count {
            threads.push(ThreadHandle::new(id));
        }

        Self {
            threads: threads.into_boxed_slice(),
        }
    }

    pub fn scope<'env, F>(&self, f: F) -> ScopeHandle<'_, 'env>
    where
        F: for<'scope> FnOnce(&'scope Scope<'scope, 'env>),
    {
        let scope = Scope {
            data: &self.threads,
            counter: Arc::new(AtomicUsize::new(0)),
            env: PhantomData,
            scope: PhantomData,
        };

        f(&scope);

        // scope should be able to be dropped here?
        // as we can't destruct scope here and take counter, clone it....
        #[allow(clippy::redundant_clone)]
        ScopeHandle {
            scope: PhantomData,
            env: PhantomData,
            counter: scope.counter.clone(),
        }
    }

    fn finish_inner(&mut self) {
        for thread in self.threads.iter() {
            thread.tx.send(Message::Finish).unwrap();
        }

        let removed = std::mem::replace(&mut self.threads, Box::new([]));

        for thread in removed {
            _ = thread.handle.join();
        }
    }
}

impl std::ops::Drop for ThreadPool {
    #[inline]
    fn drop(&mut self) {
        self.finish_inner();
    }
}

impl std::fmt::Debug for ThreadPool {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreadPool")
            .field("threads", &self.threads.len())
            .finish_non_exhaustive()
    }
}

struct ThreadHandle {
    handle: std::thread::JoinHandle<()>,
    tx: Sender<Message>,
}

impl ThreadHandle {
    fn new(id: usize) -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = std::thread::Builder::new()
            .name(format!("Pool Thread: [{id}]"))
            .spawn(move || {
                loop {
                    if let Ok(msg) = rx.recv() {
                        match msg {
                            Message::Finish => return,
                            Message::Job(job, counter) => {
                                _ = catch_unwind(AssertUnwindSafe(job));
                                counter.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                    }
                }
            })
            .unwrap();

        Self { handle, tx }
    }
}

pub struct Scope<'scope, 'env: 'scope> {
    data: &'scope [ThreadHandle],
    counter: Arc<AtomicUsize>,

    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,
}

impl<'scope, 'env> Scope<'scope, 'env> {
    #[inline]
    pub const fn thread_count(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub fn threads(
        &self,
    ) -> impl Iterator<Item = ScopedThread<'scope, 'env>> + use<'scope, 'env, '_> {
        self.data.iter().map(|handle| ScopedThread {
            counter: self.counter.clone(),
            handle,
            scope: PhantomData,
            env: PhantomData,
        })
    }
}

pub struct ScopedThread<'scope, 'env: 'scope> {
    counter: Arc<AtomicUsize>,
    handle: &'scope ThreadHandle,

    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,
}

impl<'scope, 'env: 'scope> ScopedThread<'scope, 'env> {
    pub fn run<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'env,
    {
        let job = Box::new(job);

        let task = unsafe { std::mem::transmute::<Box<Task<'env>>, Box<Task<'static>>>(job) };

        // keep track of how many jobs were send during this scope
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        self.handle
            .tx
            .send(Message::Job(task, self.counter.clone()))
            .unwrap();
    }
}

pub struct ScopeHandle<'scope, 'env: 'scope> {
    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,

    counter: Arc<AtomicUsize>,
}

impl<'scope, 'env: 'scope> ScopeHandle<'scope, 'env> {
    #[inline]
    pub fn join(self) {
        self.join_inner();
    }

    #[inline]
    fn join_inner(&self) {
        while self.counter.load(std::sync::atomic::Ordering::Relaxed) != 0 {
            std::hint::spin_loop();
        }
    }
}

impl<'scope, 'env: 'scope> std::ops::Drop for ScopeHandle<'scope, 'env> {
    #[inline]
    fn drop(&mut self) {
        self.join_inner();
    }
}

type Task<'a> = dyn FnOnce() + Send + 'a;

enum Message {
    Finish,
    Job(Box<Task<'static>>, Arc<AtomicUsize>),
}
