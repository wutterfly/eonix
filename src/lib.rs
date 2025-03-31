#![warn(clippy::nursery)]
#![warn(clippy::cargo)]

mod cells;
mod components;
mod entity;
mod query;
mod resources;
mod scene;
mod table;
mod trait_impl;
mod world;

pub use cells::AtomicRefCell;
pub use components::Component;
pub use query::Query;
pub use scene::Scene;
pub use world::World;

impl Component for u32 {}
impl Component for i32 {}
