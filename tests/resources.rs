mod common;

use eonix::World;

use common::*;

#[test]
fn test_scene_resource_insert_get() {
    let mut world = World::new();

    let scene = world.current_scene_mut();

    scene.insert_resource(R1(100));

    let res = scene.get_resource_ref::<R1>().unwrap();
    assert_eq!(&res.0, &100);
    drop(res);

    let mut res = scene.get_resource_mut::<R1>().unwrap();
    assert_eq!(&mut res.0, &mut 100);
    drop(res);
}

#[test]
fn test_scene_nosend_insert_get() {
    let mut world = World::new();

    let scene = world.current_scene_mut();

    scene.insert_nosend_resource(R2(100));

    let res = scene.get_nosend_resource_ref::<R2>().unwrap();
    assert_eq!(&res.0, &100);
    drop(res);

    let mut res = scene.get_nosend_resource_mut::<R2>().unwrap();
    assert_eq!(&mut res.0, &mut 100);
    drop(res);
}

#[test]
fn test_global_resource_insert_get() {
    let mut world = World::new();

    world.insert_resource(R1(100));

    let res = world.get_resource_ref::<R1>().unwrap();
    assert_eq!(&res.0, &100);
    drop(res);

    let mut res = world.get_resource_mut::<R1>().unwrap();
    assert_eq!(&mut res.0, &mut 100);
    drop(res);
}

#[test]
fn test_global_nosend_insert_get() {
    let mut world = World::new();

    world.insert_nosend_resource(R2(100));

    let res = world.get_nosend_resource_ref::<R2>().unwrap();
    assert_eq!(&res.0, &100);
    drop(res);

    let mut res = world.get_nosend_resource_mut::<R2>().unwrap();
    assert_eq!(&mut res.0, &mut 100);
    drop(res);
}
