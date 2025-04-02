use crate::{
    World,
    cells::split_world,
    system::{IntoSystem, StoredSystem},
    thread_pool::ThreadPool,
};

pub struct Schedule {
    systems: Vec<StoredSystem>,
    thread_pool: ThreadPool,
}

impl Schedule {
    pub fn new(threads: usize) -> Self {
        Self {
            systems: Vec::new(),
            thread_pool: ThreadPool::new(threads),
        }
    }

    pub fn add_single_system<Input: 'static>(&mut self, system: impl IntoSystem<Input> + 'static) {
        let boxed = Box::new(system.into_system());

        self.systems.push(boxed);
    }

    pub fn run(&mut self, world: &mut World) {
        let (complete, send) = split_world(world);

        self.thread_pool.scope(|s| {
            for (thread, system) in s.threads().zip(&self.systems) {
                //
                if system.local() {
                    let _ = system.run_on_main(complete.clone());
                } else {
                    thread.run(|| {
                        let _ = system.run(send.clone());
                    });
                }
            }
        });
    }
}
