use crate::{
    components::{ComponentSet, EntityComponents},
    entity::Entity,
    resources::{NoSend, Res, ResMut, Resource, Resources, Unsend, UnsendMut},
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

    pub fn spawn_entity(&mut self) -> Entity {
        self.entities.spawn_entity()
    }

    pub fn delete_entity(&mut self, entity: Entity) {
        self.entities.delete_entity(entity);
    }

    pub fn add_component<C: ComponentSet>(&mut self, entity: Entity, components: C) {
        self.entities.add_components(entity, components);
    }

    pub fn remove_components<C: ComponentSet>(&mut self, entity: &Entity) {
        self.entities.remove_component::<C>(entity);
    }

    #[inline]
    pub fn add_resource<R: Resource>(&mut self, res: R) {
        self.resources.add_resource(res);
    }

    #[inline]
    pub fn get_resource_ref<R: Resource>(&self) -> Option<Res<R>> {
        let handle = self.resources.get_resource::<R>()?;
        Some(Res { handle })
    }

    #[inline]
    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<ResMut<R>> {
        let handle = self.resources.get_resource_mut::<R>()?;
        Some(ResMut { handle })
    }

    #[inline]
    pub fn add_nosend_resource<R: NoSend>(&mut self, res: R) {
        self.nosend.add_resource(res);
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
