use std::sync::{Arc, atomic::AtomicU32};

use crossbeam_channel::{Receiver, Sender, TryRecvError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// An `Entity` is represented by a position and a generation.
///
/// An `Entity` can be understood as a column in a table, while components are rows.
pub struct Entity {
    generation: Generation,

    /// Use u32 as position here, to save space (more the u32::MAX) entities are highly unlikely.
    position: u32,
}

impl Entity {
    #[inline]
    pub(crate) const fn new(position: u32, generation: Generation) -> Self {
        Self {
            position,
            generation,
        }
    }

    #[inline]
    pub(crate) const fn generation(&self) -> Generation {
        self.generation
    }

    #[inline]
    pub(crate) const fn id(&self) -> usize {
        self.position as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A generation keeps track of entities at the same position (after deleting an `Entity` and spawning a new one).
pub struct Generation(u32);

impl Generation {
    /// Flag signaling the generation(entity) is not in use.
    const INVALID: u32 = 0b1000_0000_0000_0000_0000_0000_0000_0000;
    const VALID: u32 = 0b0111_1111_1111_1111_1111_1111_1111_1111;

    #[inline]
    /// Creates a new, valid `Generation`.
    pub const fn new() -> Self {
        Self(0)
    }

    #[inline]
    /// Creates a new, invalid `Generation`.
    pub const fn invalid() -> Self {
        Self(Self::INVALID)
    }

    #[inline]
    pub const fn set_invalid(&mut self) {
        self.0 |= Self::INVALID;
    }

    #[inline]
    pub const fn set_valid(&mut self) {
        self.0 &= Self::VALID;
    }

    #[inline]
    pub const fn is_invalid(&self) -> bool {
        self.0 & Self::INVALID != 0
    }

    #[inline]
    /// Increments the generation, wrapping when the maximum is reached.
    pub const fn inc(&mut self) {
        self.0 += 1;

        if self.is_invalid() {
            self.0 = 0;
        }
    }
}

#[derive(Debug, Clone)]
/// Allows the deferred creation/reservation of entities.
///
/// Wraps a position counter and a free-list of entities.
pub struct EntitySpawner {
    latest_entity: Arc<AtomicU32>,

    input: Sender<Entity>,
    output: Receiver<Entity>,
}

impl EntitySpawner {
    #[inline]
    pub fn new() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();

        Self {
            latest_entity: Arc::new(AtomicU32::new(0)),
            input: tx,
            output: rx,
        }
    }

    /// Reserves a `Entity`.
    ///
    /// Creates a completly new `Entity`, or reuses a `Entity` that was deleted.
    /// This only reserves the entity, to active it, call `Entities::active_entity`!
    pub fn reserve(&self) -> Entity {
        // try to receive already used entity
        match self.output.try_recv() {
            Ok(mut ent) => {
                ent.generation.set_valid();
                ent.generation.inc();
                ent
            }
            Err(TryRecvError::Disconnected) => unreachable!(),
            Err(TryRecvError::Empty) => {
                let position = self
                    .latest_entity
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                Entity::new(position, Generation::new())
            }
        }
    }

    /// Adds a given `Entity` to the free-list.
    pub fn free(&self, mut ent: Entity) {
        ent.generation.set_invalid();

        _ = self.input.send(ent);
    }
}

impl Default for EntitySpawner {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
