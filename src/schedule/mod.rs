mod builder;
mod graph;

use graph::ExecutionGraph;

pub use builder::ScheduleBuilder;

use crate::{
    World,
    cells::{WorldCellComplete, WorldCellSend, split_world},
    filter::FilterType,
    macros::catch_system_failure,
    system::{ParamType, StoredSystem},
    thread_pool::ThreadPool,
};

#[cfg_attr(feature = "debug-utils", derive(Debug))]
pub struct Schedule {
    thread_pool: ThreadPool,

    pub(crate) setup: Stage,
    pub(crate) pre_update: Stage,
    pub(crate) update: Stage,
    pub(crate) post_update: Stage,
    pub(crate) shutdown: Stage,
}

impl Schedule {
    pub fn run(&self, world: &mut World) {
        let (complete, send) = split_world(world);

        complete.borrow_mut().apply_commands();

        // start
        self.pre_update
            .run(complete.clone(), send.clone(), &self.thread_pool);

        complete.borrow_mut().apply_commands();

        // update
        self.update
            .run(complete.clone(), send.clone(), &self.thread_pool);

        complete.borrow_mut().apply_commands();

        // finish
        self.post_update
            .run(complete.clone(), send.clone(), &self.thread_pool);

        complete.borrow_mut().apply_commands();
    }

    pub fn run_setup(&self, world: &mut World) {
        let (complete, send) = split_world(world);

        self.setup.run(complete.clone(), send, &self.thread_pool);

        complete.borrow_mut().apply_commands();
    }

    pub fn run_shutdown(&self, world: &mut World) {
        let (complete, send) = split_world(world);

        self.shutdown.run(complete.clone(), send, &self.thread_pool);

        complete.borrow_mut().apply_commands();
    }
}

#[derive(Default)]
#[cfg_attr(feature = "debug-utils", derive(Debug))]
pub struct Stage {
    pub(crate) systems: ExecutionGraph,
}

impl Stage {
    pub fn run(&self, complete: WorldCellComplete, send: WorldCellSend, pool: &ThreadPool) {
        // run this stages systems
        self.systems.run(complete.clone(), send.clone(), pool);
    }
}

pub trait IntoSystemSet<Marker> {
    fn into_set(self) -> SystemSet;
}

pub enum SystemSet {
    Single { system: StoredSystem },

    Chained { systems: Box<[StoredSystem]> },
}

#[cfg(feature = "debug-utils")]
impl std::fmt::Debug for SystemSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single { system } => f.debug_struct("Single").field("system", system).finish(),
            Self::Chained { systems } => {
                f.debug_struct("Chained").field("systems", systems).finish()
            }
        }
    }
}

impl SystemSet {
    pub fn run(&self, world: WorldCellSend) {
        match self {
            // single system
            Self::Single { system } => {
                #[cfg(feature = "debug-utils")]
                catch_system_failure!(system.run(world), system.name());

                #[cfg(not(feature = "debug-utils"))]
                catch_system_failure!(system.run(world));
            }

            // multiple systems
            Self::Chained { systems } => {
                for system in systems {
                    #[cfg(feature = "debug-utils")]
                    catch_system_failure!(system.run(world.clone()), system.name());

                    #[cfg(not(feature = "debug-utils"))]
                    catch_system_failure!(system.run(world.clone()));
                }
            }
        }
    }

    pub fn run_local(&self, world: WorldCellComplete) {
        match self {
            // single system
            Self::Single { system } => {
                #[cfg(feature = "debug-utils")]
                catch_system_failure!(system.run_on_main(world), system.name());

                #[cfg(not(feature = "debug-utils"))]
                catch_system_failure!(system.run_on_main(world));
            }

            // multiple systems
            Self::Chained { systems } => {
                for system in systems {
                    #[cfg(feature = "debug-utils")]
                    catch_system_failure!(system.run_on_main(world.clone()), system.name());

                    #[cfg(not(feature = "debug-utils"))]
                    catch_system_failure!(system.run_on_main(world.clone()));
                }
            }
        }
    }

    pub fn get_info(&self) -> SetInfo {
        match self {
            Self::Single { system } => SetInfo {
                local: system.local(),
                systems: vec![SystemInfo {
                    types: system.get_types(),
                    filter: system.get_filter(),
                }],
            },
            Self::Chained { systems } => {
                let mut vec = Vec::with_capacity(systems.len());

                let mut local = false;

                for system in systems {
                    vec.push(SystemInfo {
                        types: system.get_types(),
                        filter: system.get_filter(),
                    });

                    local |= system.local();
                }

                SetInfo {
                    systems: vec,
                    local,
                }
            }
        }
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "debug-utils", derive(Debug))]
pub struct SetInfo {
    systems: Vec<SystemInfo>,
    local: bool,
}

impl SetInfo {
    #[inline]
    pub const fn local(&self) -> bool {
        self.local
    }

    #[inline]
    pub fn conflicts(&self, other: &Self) -> bool {
        for system_a in &self.systems {
            for system_b in &other.systems {
                if system_a.conflicts(system_b) {
                    return true;
                }
            }
        }

        false
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "debug-utils", derive(Debug))]
struct SystemInfo {
    types: Vec<ParamType>,
    filter: Vec<FilterType>,
}

impl SystemInfo {
    #[inline]
    fn conflicts(&self, other: &Self) -> bool {
        for type_a in &self.types {
            for type_b in &other.types {
                if type_a.conflicts(type_b) {
                    if type_a.is_world() || type_b.is_world() {
                        return true;
                    }

                    debug_assert_eq!(type_a.raw_type(), type_b.raw_type());

                    // check filters
                    if !FilterType::prevents_overlapping(&self.filter, &other.filter) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

// ################ Stages #####################

pub trait SystemStage: 'static {}

pub struct Setup;
impl SystemStage for Setup {}

pub struct PreUpdate;
impl SystemStage for PreUpdate {}

pub struct Update;
impl SystemStage for Update {}

pub struct PostUpdate;
impl SystemStage for PostUpdate {}

pub struct Shutdown;
impl SystemStage for Shutdown {}
