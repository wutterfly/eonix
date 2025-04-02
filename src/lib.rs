#![warn(clippy::nursery)]
#![warn(clippy::cargo)]

mod cells;
mod commands;
mod components;
mod entity;
mod filter;
mod macros;
mod query;
mod resources;
mod scene;
mod schedule;
mod system;
mod table;
mod thread_pool;
mod trait_impl;
mod world;

pub use cells::AtomicRefCell;
pub use commands::Commands;
pub use components::Component;
pub use entity::Entity;
pub use filter::{Or, With, WithOut};
pub use query::Query;
pub use resources::{
    GlobalRes, GlobalResMut, GlobalUnsendRef, NoSend, Res, ResMut, Resource, UnsendMut, UnsendRef,
};
pub use scene::Scene;
pub use schedule::{PostUpdate, PreUpdate, Schedule, ScheduleBuilder, Setup, Shutdown, Update};
pub use world::World;

#[cfg(feature = "derive")]
pub use eonix_derive::*;

impl Component for u32 {}
impl Component for i32 {}
