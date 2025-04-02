#![warn(clippy::nursery)]
#![warn(clippy::cargo)]

mod cells;
mod commands;
mod components;
mod entity;
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
pub use query::Query;
pub use resources::{NoSend, Resource};
pub use scene::Scene;
pub use schedule::Schedule;
pub use world::World;

impl Component for u32 {}
impl Component for i32 {}
