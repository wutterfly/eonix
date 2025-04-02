mod common;

use common::*;
use eonix::{Query, Schedule, World};

#[test]
fn test() {
    let mut world = World::new();
    let mut schedule = Schedule::new(4);

    schedule.add_single_system(system_add);
    schedule.add_single_system(system_world);

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
