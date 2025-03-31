use std::any::{Any, TypeId};

use crate::{
    components::{
        ComponentAddModifier, ComponentRemoveModifier, ComponentSet, EntityComponents,
        UntypedComponentSet,
    },
    entity::{Entity, EntitySpawner},
    resources::{
        NoSend, Res, ResMut, Resource, ResourceStorageModifier, Resources, Unsend, UnsendMut,
    },
};

pub struct Scene {
    resources: Resources<dyn Resource>,
    nosend: Resources<dyn NoSend>,

    pub(crate) entities: EntityComponents,
}

impl Scene {
    #[inline]
    pub fn new() -> Self {
        Self {
            resources: Resources::new(),
            nosend: Resources::new(),
            entities: EntityComponents::new(),
        }
    }

    #[inline]
    pub fn spawner(&self) -> EntitySpawner {
        self.entities.spawner()
    }

    pub fn spawn_entity(&mut self) -> Entity {
        self.entities.spawn_entity()
    }

    pub fn delete_entity(&mut self, entity: Entity) {
        self.entities.delete_entity(entity);
    }

    pub fn add_component<C: ComponentSet>(&mut self, entity: &Entity, components: C) {
        C::validate();
        self.entities.add_components(entity, components);
    }

    #[inline]
    pub fn add_component_untyped(
        &mut self,
        entity: &Entity,
        components: Box<UntypedComponentSet>,
        modifier: ComponentAddModifier,
    ) {
        self.entities
            .add_component_untyped(entity, components, modifier);
    }

    pub fn remove_components<C: ComponentSet>(&mut self, entity: &Entity) {
        C::validate();
        self.entities.remove_component::<C>(entity);
    }

    #[inline]
    pub fn remove_components_untyped(&mut self, entity: Entity, modifier: ComponentRemoveModifier) {
        self.entities.remove_components_untyped(&entity, modifier);
    }

    #[inline]
    pub fn insert_resource<R: Resource>(&mut self, res: R) {
        self.resources.insert_resource(res);
    }

    #[inline]
    pub fn insert_resource_untyped(
        &mut self,
        resource: Box<dyn Any>,
        modifier: ResourceStorageModifier,
    ) {
        self.resources.insert_resource_untyped(resource, modifier);
    }

    #[inline]
    pub fn get_resource_ref<R: Resource>(&self) -> Option<Res<R>> {
        let handle = self.resources.get_resource::<R>()?;
        Some(Res { handle })
    }

    #[inline]
    pub fn get_resource_mut<R: Resource>(&self) -> Option<ResMut<R>> {
        let handle = self.resources.get_resource_mut::<R>()?;
        Some(ResMut { handle })
    }

    #[inline]
    pub fn remove_resource_untyped(&mut self, type_id: TypeId) {
        self.resources.remove_resource_untyped(type_id);
    }

    #[inline]
    pub fn insert_nosend_resource<R: NoSend>(&mut self, res: R) {
        self.nosend.insert_resource(res);
    }

    #[inline]
    pub fn get_nosend_resource_ref<R: NoSend>(&self) -> Option<Unsend<R>> {
        let handle = self.nosend.get_resource::<R>()?;
        Some(Unsend { handle })
    }

    #[inline]
    pub fn get_nosend_resource_mut<R: NoSend>(&mut self) -> Option<UnsendMut<R>> {
        let handle = self.nosend.get_resource_mut::<R>()?;
        Some(UnsendMut { handle })
    }
}

impl Default for Scene {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
