use eonix::{
    Commands, Component, PostUpdate, PreUpdate, Query, Res, Resource, ScheduleBuilder, Setup,
    Shutdown, Update, World,
};

#[derive(Debug)]
struct Position(u32, u32);
impl Component for Position {}

struct Velocity(u32);
impl Component for Velocity {}

struct VelocityIncrease(u32);
impl Resource for VelocityIncrease {}

fn main() {
    let mut world = World::new();

    let schedule = ScheduleBuilder::new()
        .add_system(Setup, setup)
        .add_system(PreUpdate, before)
        .add_system(Update, (increase_velocity, update))
        .add_system(PostUpdate, after)
        .add_system(Shutdown, good_bye)
        .build();

    schedule.run_setup(&mut world);

    for _ in 0..2 {
        println!("  -------");
        schedule.run(&mut world);
        println!("  -------");
    }

    schedule.run_shutdown(&mut world);
}

fn setup(commands: Commands) {
    let ent = commands.reserve_entity();
    commands.add_component(&ent, (Position(0, 0), Velocity(0)));

    commands.add_resource(VelocityIncrease(1));
}

fn before(mut positions: Query<&Position>) {
    for pos in positions.iter() {
        println!("Before -> Position: {pos:?}");
    }
}

fn increase_velocity(mut velocities: Query<&mut Velocity>, increase: Res<VelocityIncrease>) {
    println!("  Updating velocity....");
    for vel in velocities.iter() {
        vel.0 += increase.0;
    }
}

fn update(mut positions: Query<(&mut Position, &Velocity)>) {
    println!("  Updating position....");
    for (pos, vel) in positions.iter() {
        pos.0 += vel.0;
        pos.1 += vel.0;
    }
}

fn after(mut positions: Query<&Position>) {
    for pos in positions.iter() {
        println!("After -> Position: {pos:?}");
    }
}

fn good_bye() {
    println!("Good Bye!")
}
