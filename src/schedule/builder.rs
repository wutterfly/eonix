use std::any::TypeId;

use crate::{macros::unwrap, thread_pool::ThreadPool};

use super::{
    IntoSystemSet, PostUpdate, PreUpdate, Schedule, SetInfo, Setup, Shutdown, Stage, SystemSet,
    SystemStage, Update,
    graph::{ExecutionGraph, Node},
};

#[derive(Default)]
#[cfg_attr(feature = "debug-utils", derive(Debug))]
pub struct ScheduleBuilder {
    thread_count: usize,
    max_tail: usize,

    setup: BStage,
    start: BStage,
    update: BStage,
    finish: BStage,
    shutdown: BStage,
}

impl ScheduleBuilder {
    #[inline]
    pub fn new() -> Self {
        Self {
            thread_count: 4,
            max_tail: 8,
            setup: BStage::default(),
            start: BStage::default(),
            update: BStage::default(),
            finish: BStage::default(),
            shutdown: BStage::default(),
        }
    }

    #[inline]
    pub fn build(self) -> Schedule {
        // include main thread as well
        let thread_count = self.thread_count + 1;

        // use a cached graph builder
        let mut graph_builder = GraphBuilder::new(thread_count, self.max_tail);

        Schedule {
            thread_pool: ThreadPool::new(self.thread_count),

            setup: self.setup.build(&mut graph_builder),
            pre_update: self.start.build(&mut graph_builder),
            update: self.update.build(&mut graph_builder),
            post_update: self.finish.build(&mut graph_builder),
            shutdown: self.shutdown.build(&mut graph_builder),
        }
    }

    #[inline]
    pub const fn set_thread_count(mut self, thead_count: usize) -> Self {
        self.thread_count = thead_count;

        self
    }

    #[inline]
    pub const fn set_max_tail(mut self, max_tail: usize) -> Self {
        self.max_tail = max_tail;

        self
    }

    pub fn add_system<T: SystemStage, M>(mut self, _: T, system: impl IntoSystemSet<M>) -> Self {
        let set = system.into_set();
        let stage_id = TypeId::of::<T>();

        match stage_id {
            id if id == TypeId::of::<Setup>() => self.setup.add_system(set),
            id if id == TypeId::of::<PreUpdate>() => self.start.add_system(set),
            id if id == TypeId::of::<Update>() => self.update.add_system(set),
            id if id == TypeId::of::<PostUpdate>() => self.finish.add_system(set),
            id if id == TypeId::of::<Shutdown>() => self.shutdown.add_system(set),
            _ => {
                // find substage with id
                unreachable!()
            }
        }

        self
    }
}

#[derive(Default)]
#[cfg_attr(feature = "debug-utils", derive(Debug))]
struct BStage {
    // build execution tree from these
    systems: Vec<SystemSet>,
}

impl BStage {
    fn build(self, graph_builder: &mut GraphBuilder) -> Stage {
        Stage {
            systems: graph_builder.build_graph_from(self.systems),
        }
    }

    fn add_system(&mut self, set: SystemSet) {
        self.systems.push(set);
    }
}

struct GraphBuilder {
    thread_count: usize,
    max_tail: usize,

    threads_reserved_types: Box<[Vec<SetInfo>]>,

    threads_current: Box<[Vec<Node>]>,
    threads_since_sync: Box<[usize]>,

    // stores threads that have to use a set based on their parameters (pref only one)
    conflicts: Vec<usize>,

    leftovers: Vec<SystemSet>,
}

impl GraphBuilder {
    pub fn new(thread_count: usize, max_tail: usize) -> Self {
        debug_assert_ne!(thread_count, 0);

        let mut current = Vec::with_capacity(thread_count);
        for _ in 0..thread_count {
            current.push(Vec::new());
        }

        Self {
            thread_count,
            max_tail,

            threads_reserved_types: vec![Vec::new(); thread_count].into_boxed_slice(),

            threads_current: current.into_boxed_slice(),
            threads_since_sync: vec![0; thread_count].into_boxed_slice(),
            conflicts: Vec::with_capacity(thread_count),
            leftovers: Vec::new(),
        }
    }

    pub fn build_graph_from(&mut self, mut systems: Vec<SystemSet>) -> ExecutionGraph {
        if systems.is_empty() {
            return ExecutionGraph::new_empty();
        }

        let mut tree = ExecutionGraph::new(self.thread_count);
        self.leftovers.reserve(systems.len());

        // marks the first iteration
        let mut first = true;

        while first || !systems.is_empty() {
            #[allow(clippy::iter_with_drain)]
            'inner: for system in systems.drain(..) {
                //
                let set = system.get_info();

                // check for conflicted types
                for (thread, sets) in self.threads_reserved_types.iter().enumerate() {
                    // if there is a conflict
                    if Self::check_for_type_conflict(sets, &set) {
                        self.conflicts.push(thread);
                    }
                }

                // check whether system has to run on local/main thread
                if set.local() && !self.conflicts.contains(&0) {
                    self.conflicts.push(0);
                }

                match self.conflicts.len() {
                    // no thread conflicts, choose any thread to execute it
                    0 => {
                        // get thread position (thread with the least nodes)
                        let thread_i = self.thread_min_nodes();

                        // make sure not all systems get pushed into one thread
                        if self.check_tail_too_long(thread_i) {
                            self.leftovers.push(system);
                        }
                        //
                        else {
                            self.threads_current[thread_i].push(Node::new_system(system));
                            self.threads_since_sync[thread_i] += 1;

                            // add types to reserved types for this thread
                            self.threads_reserved_types[thread_i].push(set);
                        }
                    }

                    // 1 thread conflicts, chose this thread!
                    1 => {
                        // get thread position (len == 1 => [0] is valid)
                        let thread_i = self.conflicts[0];

                        debug_assert!({
                            let info = system.get_info();
                            (info.local() && thread_i == 0) || !info.local()
                        });

                        // make sure not all systems get pushed into one thread
                        if self.check_tail_too_long(thread_i) {
                            self.leftovers.push(system);
                        } else {
                            self.threads_current[thread_i].push(Node::new_system(system));
                            self.threads_since_sync[thread_i] += 1;

                            // add types to reserved types for this thread
                            self.threads_reserved_types[thread_i].push(set);
                        }
                    }

                    // more then 1 thread conflicts, this system can't be run at this point
                    _ => {
                        // system does not fit
                        // store system for next round, try next system
                        self.leftovers.push(system);
                    }
                }

                self.conflicts.clear();

                continue 'inner;
            }

            // finished checking all systems, move the leftover back
            std::mem::swap(&mut systems, &mut self.leftovers);

            // insert collected systems into graph
            for (thread_i, thread) in self.threads_current.iter_mut().enumerate() {
                for set in thread.drain(..) {
                    Self::add_node_for_thread(&mut tree, thread_i, set);
                }
            }

            // insert sync point
            Self::add_sync_for_all(&mut tree, self.thread_count);
            self.clear_since_sync();

            // clear thread reserved types (sync point prevents conflicts with previous param types)
            self.clear_threads_reserved();

            first = false;
        }

        // reset builder
        self.clear_threads_reserved();
        self.clear_since_sync();

        debug_assert!(self.conflicts.is_empty());
        debug_assert!(self.leftovers.is_empty());
        debug_assert!(systems.is_empty());
        for tc in &self.threads_current {
            debug_assert!(tc.is_empty());
        }

        tree
    }

    fn check_tail_too_long(&self, thread_i: usize) -> bool {
        // guaranteed to have value initialized for each thread
        // so can unwrap here
        let (t_max, max) = unwrap!(
            self.threads_since_sync
                .iter()
                .enumerate()
                .max_by(|(_, x), (_, y)| x.cmp(y))
        );

        if thread_i != t_max {
            return false;
        }

        // guaranteed to have value initialized for each thread
        // so can unwrap here
        let min = unwrap!(self.threads_since_sync.iter().min_by(|x, y| x.cmp(y)));

        max - min >= self.max_tail
    }

    #[inline]
    fn add_sync_for_all(tree: &mut ExecutionGraph, thread_count: usize) {
        let barrier = Node::create_barrier(thread_count);
        for root in &mut tree.node_tree {
            root.append_last(&mut tree.nodes, Node::new_sync(barrier.clone()));
        }
    }

    #[inline]
    fn thread_min_nodes(&self) -> usize {
        self.threads_current
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.len().cmp(&b.len()))
            .map(|(i, _)| i)
            .unwrap()
    }

    #[inline]
    fn add_node_for_thread(tree: &mut ExecutionGraph, thread_i: usize, node: Node) {
        tree.node_tree[thread_i].append_last(&mut tree.nodes, node);
    }

    fn check_for_type_conflict(stored_sets: &Vec<SetInfo>, set: &SetInfo) -> bool {
        for stored_set in stored_sets {
            if stored_set.conflicts(set) {
                return true;
            }
        }

        false
    }

    #[inline]
    fn clear_threads_reserved(&mut self) {
        for reserved in &mut self.threads_reserved_types {
            reserved.clear();
        }
    }

    #[inline]
    fn clear_since_sync(&mut self) {
        for x in &mut self.threads_since_sync {
            *x = 0;
        }
    }
}

#[cfg(test)]
mod tests {

    mod stages {

        pub use super::super::*;

        fn sys(_: crate::Query<&mut u32>) {}

        #[test]
        fn test_add_inbuild_system() {
            let schedule = ScheduleBuilder::new();

            let schedule = schedule.add_system(PreUpdate, sys);
            assert_eq!(schedule.start.systems.len(), 1);

            let schedule = schedule.add_system(Update, sys);
            assert_eq!(schedule.update.systems.len(), 1);

            let schedule = schedule.add_system(PostUpdate, sys);
            assert_eq!(schedule.finish.systems.len(), 1);

            let _ = schedule.build();
        }

        #[test]
        fn test_builder_add_system() {
            let schedule = ScheduleBuilder::new();

            //

            let schedule = schedule.add_system(PreUpdate, sys);

            assert_eq!(schedule.start.systems.len(), 1);
            assert_eq!(schedule.update.systems.len(), 0);
            assert_eq!(schedule.finish.systems.len(), 0);

            //

            let schedule = schedule.add_system(Update, sys);

            assert_eq!(schedule.start.systems.len(), 1);
            assert_eq!(schedule.update.systems.len(), 1);
            assert_eq!(schedule.finish.systems.len(), 0);

            //

            let schedule = schedule.add_system(PostUpdate, sys);

            assert_eq!(schedule.start.systems.len(), 1);
            assert_eq!(schedule.update.systems.len(), 1);
            assert_eq!(schedule.finish.systems.len(), 1);
        }
    }

    mod builder {
        use crate::{Query, With, WithOut};

        pub use super::super::*;

        const THREAD_COUNT: usize = 4;
        const MAX_TAIL: usize = 3;

        fn sys_ref_u32(_: Query<&u32>) {}

        fn sys_mut_u32(_: Query<&mut u32>) {}

        fn sys_ref_i32(_: Query<&i32>) {}

        fn sys_mut_i32(_: Query<&mut i32>) {}

        fn sys_ref_shared(_: Query<(&i32, &u32)>) {}

        fn sys_world(_: &mut crate::World) {}

        fn sys_mut_u32_not_i32(_: Query<&mut u32, WithOut<i32>>) {}

        fn sys_mut_u32_with_i32(_: Query<&mut u32, With<i32>>) {}

        #[test]
        fn test_builder_empty() {
            let builder = ScheduleBuilder::new().set_thread_count(THREAD_COUNT);
            assert_eq!(builder.thread_count, THREAD_COUNT);

            let schedule = builder.build();

            assert_eq!(schedule.update.systems.len(), 0);
        }

        #[test]
        fn test_builder_single_system() {
            let builder = ScheduleBuilder::new()
                .set_thread_count(THREAD_COUNT)
                .set_max_tail(MAX_TAIL);
            assert_eq!(builder.thread_count, THREAD_COUNT);
            assert_eq!(builder.max_tail, MAX_TAIL);

            // [sys_ref_i32, SYNC]
            // [SYNC]
            // [SYNC]
            // [SYNC]
            let builder = builder.add_system(Update, sys_ref_i32);
            let schedule = builder.build();

            assert_eq!(schedule.update.systems.node_tree[0].node_count, 2);
            assert_eq!(schedule.update.systems.node_tree[1].node_count, 1);
            assert_eq!(schedule.update.systems.node_tree[2].node_count, 1);
            assert_eq!(schedule.update.systems.node_tree[3].node_count, 1);
        }

        #[test]
        fn test_builder_system_dependens() {
            let builder = ScheduleBuilder::new()
                .set_thread_count(THREAD_COUNT)
                .set_max_tail(MAX_TAIL);
            assert_eq!(builder.thread_count, THREAD_COUNT);
            assert_eq!(builder.max_tail, MAX_TAIL);

            // [sys_ref_i32, sys_mut_i32, SYNC]
            // [                          SYNC]
            // [                          SYNC]
            // [                          SYNC]
            let builder = builder
                .add_system(Update, sys_ref_i32)
                .add_system(Update, sys_mut_i32);
            let schedule = builder.build();

            assert_eq!(schedule.update.systems.node_tree[0].node_count, 3);
            assert_eq!(schedule.update.systems.node_tree[1].node_count, 1);
            assert_eq!(schedule.update.systems.node_tree[2].node_count, 1);
            assert_eq!(schedule.update.systems.node_tree[3].node_count, 1);
        }

        #[test]
        fn test_builder_system_independens() {
            let builder = ScheduleBuilder::new()
                .set_thread_count(THREAD_COUNT)
                .set_max_tail(MAX_TAIL);
            assert_eq!(builder.thread_count, THREAD_COUNT);
            assert_eq!(builder.max_tail, MAX_TAIL);

            // [sys_ref_i32, SYNC]
            // [sys_ref_i32, SYNC]
            // [sys_ref_u32, SYNC]
            // [sys_ref_u32, SYNC]
            let builder = builder
                .add_system(Update, sys_ref_i32)
                .add_system(Update, sys_ref_i32)
                .add_system(Update, sys_ref_u32)
                .add_system(Update, sys_ref_u32);
            let schedule = builder.build();

            assert_eq!(schedule.update.systems.node_tree[0].node_count, 2);
            assert_eq!(schedule.update.systems.node_tree[1].node_count, 2);
            assert_eq!(schedule.update.systems.node_tree[2].node_count, 2);
            assert_eq!(schedule.update.systems.node_tree[3].node_count, 2);
        }

        #[test]
        fn test_builder_system_mixed_dependens() {
            let builder = ScheduleBuilder::new()
                .set_thread_count(THREAD_COUNT)
                .set_max_tail(MAX_TAIL);
            assert_eq!(builder.thread_count, THREAD_COUNT);
            assert_eq!(builder.max_tail, MAX_TAIL);

            // [sys_ref_i32, sys_mut_i32, SYNC]
            // [sys_ref_u32, sys_mut_u32, SYNC]
            // [                          SYNC]
            // [                          SYNC]
            let builder = builder
                .add_system(Update, sys_ref_i32)
                .add_system(Update, sys_mut_i32)
                .add_system(Update, sys_ref_u32)
                .add_system(Update, sys_mut_u32);
            let schedule = builder.build();

            assert_eq!(schedule.update.systems.node_tree[0].node_count, 3);
            assert_eq!(schedule.update.systems.node_tree[1].node_count, 3);
            assert_eq!(schedule.update.systems.node_tree[2].node_count, 1);
            assert_eq!(schedule.update.systems.node_tree[3].node_count, 1);
        }

        #[test]
        fn test_builder_system_mixed_shared_dependens() {
            let builder = ScheduleBuilder::new()
                .set_thread_count(THREAD_COUNT)
                .set_max_tail(MAX_TAIL);
            assert_eq!(builder.thread_count, THREAD_COUNT);
            assert_eq!(builder.max_tail, MAX_TAIL);

            // [sys_ref_i32, sys_mut_i32, SYNC1, sys_ref_shared, SYNC2]
            // [sys_ref_u32, sys_mut_u32, SYNC1,                 SYNC2]
            // [                          SYNC1,                 SYNC2]
            // [                          SYNC1,                 SYNC2]
            let builder = builder
                .add_system(Update, sys_ref_i32)
                .add_system(Update, sys_mut_i32)
                .add_system(Update, sys_ref_u32)
                .add_system(Update, sys_mut_u32)
                .add_system(Update, sys_ref_shared);
            let schedule = builder.build();

            assert_eq!(schedule.update.systems.node_tree[0].node_count, 5);
            assert_eq!(schedule.update.systems.node_tree[1].node_count, 4);
            assert_eq!(schedule.update.systems.node_tree[2].node_count, 2);
            assert_eq!(schedule.update.systems.node_tree[3].node_count, 2);
        }

        #[test]
        fn test_builder_system_mixed_shared_dependens_max_tail() {
            let builder = ScheduleBuilder::new()
                .set_thread_count(THREAD_COUNT)
                .set_max_tail(MAX_TAIL);
            assert_eq!(builder.thread_count, THREAD_COUNT);
            assert_eq!(builder.max_tail, MAX_TAIL);

            // [sys_ref_shared, sys_mut_i32, sys_mut_u32, SYNC1, sys_ref_i32, SYNC2]
            // [                                          SYNC1, sys_ref_u32, SYNC2]
            // [                                          SYNC1,              SYNC2]
            // [                                          SYNC1,              SYNC2]
            let builder = builder
                .add_system(Update, sys_ref_shared)
                .add_system(Update, sys_mut_i32)
                .add_system(Update, sys_mut_u32)
                .add_system(Update, sys_ref_i32)
                .add_system(Update, sys_ref_u32);
            let schedule = builder.build();

            assert_eq!(schedule.update.systems.node_tree[0].node_count, 6);
            assert_eq!(schedule.update.systems.node_tree[1].node_count, 3);
            assert_eq!(schedule.update.systems.node_tree[2].node_count, 2);
            assert_eq!(schedule.update.systems.node_tree[3].node_count, 2);
        }

        #[test]
        fn test_builder_system_world() {
            let builder = ScheduleBuilder::new()
                .set_thread_count(THREAD_COUNT)
                .set_max_tail(MAX_TAIL);
            assert_eq!(builder.thread_count, THREAD_COUNT);
            assert_eq!(builder.max_tail, MAX_TAIL);

            // [sys_mut_i32, sys_ref_i32, SYNC1, sys_world, SYNC2]
            // [sys_mut_u32, sys_ref_u32, SYNC1,            SYNC2]
            // [                          SYNC1,            SYNC2]
            // [                          SYNC1,            SYNC2]
            let builder = builder
                .add_system(Update, sys_mut_i32)
                .add_system(Update, sys_mut_u32)
                .add_system(Update, sys_world)
                .add_system(Update, sys_ref_i32)
                .add_system(Update, sys_ref_u32);
            let schedule = builder.build();

            assert_eq!(schedule.update.systems.node_tree[0].node_count, 5);
            assert_eq!(schedule.update.systems.node_tree[1].node_count, 4);
            assert_eq!(schedule.update.systems.node_tree[2].node_count, 2);
            assert_eq!(schedule.update.systems.node_tree[3].node_count, 2);
        }

        #[test]
        fn test_builder_mix_shared_depend_filter() {
            let builder = ScheduleBuilder::new()
                .set_thread_count(THREAD_COUNT)
                .set_max_tail(MAX_TAIL);
            assert_eq!(builder.thread_count, THREAD_COUNT);
            assert_eq!(builder.max_tail, MAX_TAIL);

            // [sys_mut_u32_not_i32 , SYNC1]
            // [sys_mut_u32_with_i32, SYNC1]
            // [                      SYNC1]
            // [                      SYNC1]
            let builder = builder
                .add_system(Update, sys_mut_u32_not_i32)
                .add_system(Update, sys_mut_u32_with_i32);
            let schedule = builder.build();

            assert_eq!(schedule.update.systems.node_tree[0].node_count, 2);
            assert_eq!(schedule.update.systems.node_tree[1].node_count, 2);
            assert_eq!(schedule.update.systems.node_tree[2].node_count, 1);
            assert_eq!(schedule.update.systems.node_tree[3].node_count, 1);
        }

        #[test]
        fn test_builder_mix_shared_depend_filter2() {
            let builder = ScheduleBuilder::new()
                .set_thread_count(THREAD_COUNT)
                .set_max_tail(MAX_TAIL);
            assert_eq!(builder.thread_count, THREAD_COUNT);
            assert_eq!(builder.max_tail, MAX_TAIL);

            // [sys_mut_u32_not_i32 , sys_mut_u32_not_i32,  SYNC1, sys_world , SYNC2]
            // [sys_mut_u32_with_i32,                       SYNC1            , SYNC2]
            // [                                            SYNC1            , SYNC2]
            // [                                            SYNC1            , SYNC2]
            let builder = builder
                .add_system(Update, sys_mut_u32_not_i32)
                .add_system(Update, sys_mut_u32_with_i32)
                .add_system(Update, sys_mut_u32_not_i32)
                .add_system(Update, sys_world);
            let schedule = builder.build();

            assert_eq!(schedule.update.systems.node_tree[0].node_count, 5);
            assert_eq!(schedule.update.systems.node_tree[1].node_count, 3);
            assert_eq!(schedule.update.systems.node_tree[2].node_count, 2);
            assert_eq!(schedule.update.systems.node_tree[3].node_count, 2);
        }
    }
}
