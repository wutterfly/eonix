use crate::{
    commands::{CommandCenter, Commands, ComponentCommands, EntityCommands, ResourceCommands},
    resources::{
        GlobalRes, GlobalResMut, GlobalUnsendMut, GlobalUnsendRef, NoSend, Resource, Resources,
    },
    scene::{Scene, SendScene, SendScene2},
};

#[cfg_attr(feature = "debug-utils", derive(Debug))]
pub struct World {
    pub(crate) commands: CommandCenter,

    global_resources: Resources<dyn Resource>,
    global_nosend: Resources<dyn NoSend>,

    current_scene: Scene,
}

#[cfg(not(feature = "debug-utils"))]
impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "World")
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            commands: CommandCenter::new(),
            global_resources: Resources::new(),
            global_nosend: Resources::new(),
            current_scene: Scene::new(),
        }
    }

    #[inline]
    pub const fn current_scene(&self) -> &Scene {
        &self.current_scene
    }

    #[inline]
    pub const fn current_scene_mut(&mut self) -> &mut Scene {
        &mut self.current_scene
    }

    #[inline]
    pub const fn global_resources(&self) -> &Resources<dyn Resource> {
        &self.global_resources
    }

    #[inline]
    pub const fn global_nosend(&self) -> &Resources<dyn NoSend> {
        &self.global_nosend
    }

    #[inline]
    pub fn commands(&self) -> Commands {
        self.commands.commands(self.current_scene.spawner())
    }

    #[inline]
    pub fn insert_resource<R: Resource>(&mut self, res: R) {
        self.global_resources.insert_resource(res);
    }

    #[inline]
    pub fn get_resource_ref<R: Resource>(&self) -> Option<GlobalRes<R>> {
        let handle = self.global_resources.get_resource_ref::<R>()?.into();
        Some(handle)
    }

    #[inline]
    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<GlobalResMut<R>> {
        let handle = self.global_resources.get_resource_mut::<R>()?;
        Some(GlobalResMut { handle })
    }

    #[inline]
    pub fn insert_nosend_resource<R: NoSend>(&mut self, res: R) {
        self.global_nosend.insert_resource(res);
    }

    #[inline]
    pub fn get_nosend_resource_ref<R: NoSend>(&self) -> Option<GlobalUnsendRef<R>> {
        let handle = self.global_nosend.get_resource_ref::<R>()?.into();
        Some(handle)
    }

    #[inline]
    pub fn get_nosend_resource_mut<R: NoSend>(&mut self) -> Option<GlobalUnsendMut<R>> {
        let handle = self.global_nosend.get_resource_mut::<R>()?;
        Some(GlobalUnsendMut { handle })
    }

    #[inline]
    /// Executes all deferred commands.
    pub fn apply_commands(&mut self) {
        self.apply_entity_commands();
        self.apply_component_commands();
        self.apply_resource_commands();
    }

    fn apply_entity_commands(&mut self) {
        let cmds = self.commands.entity_commands();

        for cmd in cmds {
            match cmd {
                EntityCommands::SpawnEntity(entity) => {
                    self.current_scene.entities.activate_entity(entity)
                }
                EntityCommands::DeleteEntity(entity) => {
                    self.current_scene.entities.delete_entity(entity)
                }
            }
        }
    }

    fn apply_component_commands(&mut self) {
        let cmds = self.commands.component_commands();

        for cmd in cmds {
            match cmd {
                ComponentCommands::AddComponent {
                    entity,
                    components,
                    producer,
                } => {
                    self.current_scene
                        .add_component_untyped(&entity, components, (producer)());
                }
                ComponentCommands::RemoveComponent { entity, modifier } => {
                    self.current_scene
                        .remove_components_untyped(entity, (modifier)());
                }
            }
        }
    }

    fn apply_resource_commands(&mut self) {
        let cmds = self.commands.resource_commands();

        for cmd in cmds {
            match cmd {
                ResourceCommands::AddResource { resource, producer } => {
                    //
                    self.current_scene
                        .insert_resource_untyped(resource, producer);
                }
                ResourceCommands::RemoveResource { type_id } => {
                    //
                    self.current_scene.remove_resource_untyped(type_id);
                }
            }
        }
    }

    #[inline]
    pub(crate) const fn send_world(&self) -> SendWorld {
        SendWorld {
            commands: &self.commands,
            scene: self.current_scene.send_scene(),
            global_resource: &self.global_resources,
        }
    }

    pub(crate) const fn send_world2(&self) -> SendWorldPtr {
        SendWorldPtr {
            commands: &self.commands,
            scene: self.current_scene().send_scene2(),
            global_resource: self.global_resources(),
        }
    }
}

impl Default for World {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

pub struct SendWorld<'a> {
    pub(crate) commands: &'a CommandCenter,
    pub(crate) scene: SendScene<'a>,
    pub(crate) global_resource: &'a Resources<dyn Resource>,
}

#[derive(Debug, Clone, Copy)]
pub struct SendWorldPtr<'a> {
    pub(crate) commands: *const CommandCenter,
    pub(crate) scene: SendScene2<'a>,
    pub(crate) global_resource: *const Resources<dyn Resource>,
}

unsafe impl Send for SendWorldPtr<'_> {}
unsafe impl Sync for SendWorldPtr<'_> {}

impl SendWorldPtr<'_> {
    #[inline]
    pub const fn send_world(&self) -> SendWorld {
        SendWorld {
            commands: unsafe { self.commands.as_ref() }.unwrap(),
            scene: self.scene.send_scene(),
            global_resource: unsafe { self.global_resource.as_ref() }.unwrap(),
        }
    }
}
