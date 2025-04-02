use std::sync::{Arc, Barrier};

use crate::{
    cells::{WorldCellComplete, WorldCellSend},
    macros::unwrap,
    thread_pool::ThreadPool,
};

use super::SystemSet;

#[derive(Default)]
pub struct ExecutionGraph {
    pub(super) node_tree: Box<[Root]>,
    pub(super) nodes: Vec<Node>,
}

#[cfg(feature = "debug-utils")]
impl std::fmt::Debug for ExecutionGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        let mut string = String::with_capacity(1024);

        _ = writeln!(string, "ExecutionTree");

        for x in &self.node_tree {
            // get first node
            let mut node: Option<&Node> = x.node.map(|i| &self.nodes[i]);

            // keep walking the linked-list
            while let Some(n) = node {
                _ = write!(string, "\n    {:?} ->", n);

                node = n.next(&self.nodes);
            }

            _ = string.write_str("|\n");
        }

        f.write_str(&string)
    }
}

impl ExecutionGraph {
    #[inline]
    pub fn new(thread_count: usize) -> Self {
        debug_assert_ne!(thread_count, 0);

        Self {
            node_tree: vec![Root::new(); thread_count].into_boxed_slice(),
            nodes: Vec::new(),
        }
    }

    pub fn new_empty() -> Self {
        Self {
            node_tree: Box::new([]),
            nodes: Vec::new(),
        }
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn run(&self, complete: WorldCellComplete, send: WorldCellSend, pool: &ThreadPool) {
        if self.is_empty() {
            return;
        }

        let handle = pool.scope(|s| {
            // skip first element here, as it has to run localy
            let iter = self.node_tree.iter().skip(1);

            // send every root node to a thread to execute
            // number of threads and number of root nodes should match
            debug_assert_eq!(self.node_tree.len(), s.thread_count() + 1);
            for (root, thread) in iter.zip(s.threads()) {
                let world = send.clone();
                thread.run(|| {
                    root.run(world, &self.nodes);
                });
            }

            self.node_tree[0].run_local(complete, &self.nodes);
        });

        handle.join();
    }
}

#[derive(Debug, Default, Clone)]
pub struct Root {
    node: Option<usize>,
    last_node: usize,
    pub(super) node_count: usize,
}

impl Root {
    #[inline]
    const fn new() -> Self {
        Self {
            node: None,
            last_node: 0,
            node_count: 0,
        }
    }

    pub(crate) fn append_last<'a>(&mut self, nodes: &'a mut Vec<Node>, node: Node) -> &'a mut Node {
        // store new position
        let position = nodes.len();

        // push node to position
        nodes.push(node);

        self.node_count += 1;

        // append node
        if self.node.is_some() {
            // We can unwrap here, because last_node is always a valid index
            let last = unwrap!(nodes.get_mut(self.last_node));
            last.set_next(position);
        }
        // insert first node
        else {
            self.node = Some(position);
        }

        self.last_node = position;
        // We can unwrap here, because last_node is always a valid index
        unwrap!(nodes.get_mut(position))
    }

    fn run(&self, world: WorldCellSend, nodes: &[Node]) {
        // get first node
        let mut node: Option<&Node> = self.node.map(|i| &nodes[i]);

        // keep walking the linked-list
        while let Some(n) = node {
            n.run(world.clone());

            node = n.next(nodes);
        }
    }

    fn run_local(&self, world: WorldCellComplete, nodes: &[Node]) {
        // get first node
        let mut node: Option<&Node> = self.node.map(|i| &nodes[i]);

        // keep walking the linked-list
        while let Some(n) = node {
            n.run_local(world.clone());

            node = n.next(nodes);
        }
    }
}

#[cfg_attr(feature = "debug-utils", derive(Debug))]
pub enum Node {
    System {
        next: Option<usize>,
        systems: SystemSet,
    },
    Sync {
        barrier: SyncPoint,
        next: Option<usize>,
    },
}

impl Node {
    #[inline]
    pub(crate) const fn new_system(systems: SystemSet) -> Self {
        Self::System {
            next: None,
            systems,
        }
    }

    #[inline]
    pub(crate) fn create_barrier(thread_count: usize) -> SyncPoint {
        SyncPoint::new(thread_count)
    }

    #[inline]
    pub(crate) const fn new_sync(barrier: SyncPoint) -> Self {
        Self::Sync {
            barrier,
            next: None,
        }
    }

    #[inline]
    fn run(&self, world: WorldCellSend) {
        match self {
            Self::System { systems, .. } => systems.run(world),
            Self::Sync { barrier, .. } => barrier.wait(),
        }
    }

    #[inline]
    fn run_local(&self, world: WorldCellComplete) {
        match self {
            Self::System { systems, .. } => systems.run_local(world),
            Self::Sync { barrier, .. } => barrier.wait(),
        }
    }

    #[inline]
    fn next<'a>(&self, nodes: &'a [Self]) -> Option<&'a Self> {
        match self {
            Self::System { next, .. } => next.map(|i| &nodes[i]),
            Self::Sync { next, .. } => next.map(|i| &nodes[i]),
        }
    }

    #[inline]
    const fn set_next(&mut self, n: usize) {
        match self {
            Self::System { next, .. } | Self::Sync { next, .. } => *next = Some(n),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SyncPoint {
    barrier: Arc<Barrier>,
}

impl SyncPoint {
    fn new(threads: usize) -> Self {
        Self {
            barrier: Arc::new(Barrier::new(threads)),
        }
    }

    fn wait(&self) {
        let _ = self.barrier.wait();
    }
}
