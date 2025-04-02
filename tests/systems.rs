mod common;

use common::*;
use eonix::{Query, ScheduleBuilder, Update, World};

#[test]
fn test() {
    let mut world = World::new();
    let schedule = ScheduleBuilder::new()
        .add_system(Update, system_add)
        .add_system(Update, system_world)
        .build();

    schedule.run(&mut world);
}

fn system_add(mut query: Query<&mut C1>) {
    for c1 in query.iter() {
        c1.0 += 1;
    }
}

fn system_world(world: &mut World) {
    world.apply_commands();
}
