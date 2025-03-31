use std::any::TypeId;

use crossbeam_channel::{Receiver, Sender, unbounded};

use crate::{
    Entity, Resource,
    components::{
        ComponentAddModifier, ComponentRemoveModifier, ComponentSet, UntypedComponentSet,
    },
    entity::EntitySpawner,
    resources::{ResourceStorageModifier, UntypedResource},
};

#[derive(Debug)]
/// A struct holding all command queues.
///
/// Allows the creation of a `Commands` struct.
pub struct CommandCenter {
    // entities
    entity_sender: Sender<EntityCommands>,
    entity_receiver: Receiver<EntityCommands>,

    // components
    component_sender: Sender<ComponentCommands>,
    component_receiver: Receiver<ComponentCommands>,
    // resources
    resource_sender: Sender<ResourceCommands>,
    resource_receiver: Receiver<ResourceCommands>,
}

impl CommandCenter {
    #[inline]
    /// Creates a new command center.
    ///
    /// Needs a `EntitiySpawner` to allow the `Commands` struct to spawn entities.
    pub fn new() -> Self {
        let (entity_tx, entity_rx) = unbounded();
        let (component_tx, component_rx) = unbounded();
        let (resource_tx, resource_rx) = unbounded();

        Self {
            entity_sender: entity_tx,
            entity_receiver: entity_rx,

            component_sender: component_tx,
            component_receiver: component_rx,

            resource_sender: resource_tx,
            resource_receiver: resource_rx,
        }
    }

    #[inline]
    /// Creates a `Commands` struct.
    pub fn commands(&self, spawner: EntitySpawner) -> Commands {
        Commands {
            entity_sender: self.entity_sender.clone(),
            spawner,
            component_sender: self.component_sender.clone(),
            resource_sender: self.resource_sender.clone(),
        }
    }

    #[inline]
    /// Returns an iterator over all stored commands relating to entities.
    pub fn entity_commands(&self) -> impl Iterator<Item = EntityCommands> + '_ {
        self.entity_receiver.try_iter()
    }

    #[inline]
    /// Returns an iterator over all stored commands relating to components.
    pub fn component_commands(&self) -> impl Iterator<Item = ComponentCommands> + '_ {
        self.component_receiver.try_iter()
    }

    #[inline]
    /// Returns an iterator over all stored commands relating to resources.
    pub fn resource_commands(&self) -> impl Iterator<Item = ResourceCommands> + '_ {
        self.resource_receiver.try_iter()
    }
}

#[derive(Debug)]
/// A struct that allows the dispatch of different commands.
///
/// Commands are applied deferred.
pub struct Commands {
    // entites
    entity_sender: Sender<EntityCommands>,
    spawner: EntitySpawner,

    // components
    component_sender: Sender<ComponentCommands>,

    // resources
    resource_sender: Sender<ResourceCommands>,
}

impl Commands {
    #[inline]
    /// Spawns a new `Entity`.
    ///
    /// The returned `Entity` can be used (for example to add components), but is not yet valid.
    pub fn reserve_entity(&self) -> Entity {
        let entity = self.spawner.reserve();

        _ = self.entity_sender.send(EntityCommands::SpawnEntity(entity));

        entity
    }

    #[inline]
    /// Deletes an `Entity`.
    ///
    /// Deleting an `Entity` deletes all associated components as well.
    pub fn delete_entity(&self, entity: Entity) {
        _ = self
            .entity_sender
            .send(EntityCommands::DeleteEntity(entity));
    }

    #[inline]
    /// Addes a component to a given `Entity`.
    pub fn add_component<C: ComponentSet>(&self, entity: &Entity, component: C) {
        _ = self.component_sender.send(ComponentCommands::AddComponent {
            entity: *entity,
            components: Box::new(component),
            producer: ComponentAddModifier::new::<C>,
        });
    }

    #[inline]
    /// Removes a component from a given `Entity`.
    pub fn remove_component<C: ComponentSet>(&self, entity: &Entity) {
        _ = self
            .component_sender
            .send(ComponentCommands::RemoveComponent {
                entity: *entity,
                modifier: ComponentRemoveModifier::new::<C>,
            });
    }

    #[inline]
    /// Adds a new resource.
    pub fn add_resource<R: Resource>(&self, resource: R) {
        _ = self.resource_sender.send(ResourceCommands::AddResource {
            resource: Box::new(resource),
            producer: ResourceStorageModifier::new::<R>(),
        })
    }

    #[inline]
    /// Removes a resource.
    pub fn remove_resource<R: Resource>(&self) {
        _ = self.resource_sender.send(ResourceCommands::RemoveResource {
            type_id: TypeId::of::<R>(),
        })
    }
}

#[derive(Debug)]
/// Different kind of `Entity` commands.
pub enum EntityCommands {
    SpawnEntity(Entity),
    DeleteEntity(Entity),
}

#[derive(Debug)]
/// Different kind of component commands.
pub enum ComponentCommands {
    AddComponent {
        entity: Entity,
        components: Box<UntypedComponentSet>,
        producer: fn() -> ComponentAddModifier,
    },
    RemoveComponent {
        entity: Entity,
        modifier: fn() -> ComponentRemoveModifier,
    },
}

#[derive(Debug)]
/// Different kind of resource commands.
pub enum ResourceCommands {
    AddResource {
        resource: Box<UntypedResource>,
        producer: ResourceStorageModifier,
    },
    RemoveResource {
        type_id: TypeId,
    },
}
