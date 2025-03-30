use crate::{
    resources::{
        GlobalRes, GlobalResMut, GlobalUnsend, GlobalUnsendMut, NoSend, Resource, Resources,
    },
    scene::Scene,
};

pub struct World {
    global_resources: Resources<dyn Resource>,
    global_nosend: Resources<dyn NoSend>,

    current_scene: Scene,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            global_resources: Resources::new(),
            global_nosend: Resources::new(),
            current_scene: Scene::new(),
        }
    }

    pub const fn current_scene(&self) -> &Scene {
        &self.current_scene
    }

    pub const fn current_scene_mut(&mut self) -> &mut Scene {
        &mut self.current_scene
    }

    #[inline]
    pub fn get_resource_ref<R: Resource>(&self) -> Option<GlobalRes<R>> {
        let handle = self.global_resources.get_resource::<R>()?;
        Some(GlobalRes { handle })
    }

    #[inline]
    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<GlobalResMut<R>> {
        let handle = self.global_resources.get_resource_mut::<R>()?;
        Some(GlobalResMut { handle })
    }

    #[inline]
    pub fn add_nosend_resource<R: NoSend>(&mut self, res: R) {
        self.global_nosend.add_resource(Box::new(res));
    }

    #[inline]
    pub fn get_nosend_resource_ref<R: NoSend>(&self) -> Option<GlobalUnsend<R>> {
        let handle = self.global_nosend.get_resource::<R>()?;
        Some(GlobalUnsend { handle })
    }

    #[inline]
    pub fn get_nosend_resource_mut<R: NoSend>(&mut self) -> Option<GlobalUnsendMut<R>> {
        let handle = self.global_nosend.get_resource_mut::<R>()?;
        Some(GlobalUnsendMut { handle })
    }
}
