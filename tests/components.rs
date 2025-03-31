mod common;

use eonix::{Query, World};

use common::*;

#[test]
fn test_query_get() {
    let mut world = World::new();

    let scene = world.current_scene_mut();
    let mut ents = Vec::with_capacity(100);

    for i in 0..100 {
        let entity = scene.spawn_entity();
        scene.add_component(&entity, (C1(i), C2(i + 100)));
        ents.push(entity);
    }

    let mut query = Query::<(&C1, &mut C2)>::new(&scene).unwrap();
    assert_eq!(query.table_count(), 1);

    for (i, ent) in ents.iter().enumerate() {
        let (c1, c2) = query.get_entity_components(ent).unwrap();
        assert_eq!(c1.0, i as u32);
        assert_eq!(c2.0, i as u32 + 100);
    }
}

#[test]
fn test_add_components() {
    let mut world = World::new();

    let scene = world.current_scene_mut();
    let entity = scene.spawn_entity();

    //

    scene.add_component(&entity, (C1(42), C2(123)));

    {
        let mut query = Query::<&C1>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity).unwrap();
        assert_eq!(*res, C1(42));

        let mut query = Query::<&mut C2>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity).unwrap();
        assert_eq!(*res, C2(123));
    }

    //

    scene.add_component(&entity, C1(1002));

    {
        let mut query = Query::<&mut C1>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity).unwrap();
        assert_eq!(*res, C1(1002));

        let mut query = Query::<&C2>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity).unwrap();
        assert_eq!(*res, C2(123));
    }

    //

    scene.add_component(&entity, C3(090));

    {
        let mut query = Query::<&C1>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity).unwrap();
        assert_eq!(*res, C1(1002));

        let mut query = Query::<&C2>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity).unwrap();
        assert_eq!(*res, C2(123));

        let mut query = Query::<&mut C3>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity).unwrap();
        assert_eq!(*res, C3(090));
    }
}

#[test]
fn test_remove_components() {
    let mut world = World::new();

    let scene = world.current_scene_mut();
    let entity = scene.spawn_entity();
    scene.add_component(&entity, C1(001));
    scene.add_component(&entity, C2(002));
    scene.add_component(&entity, C3(003));

    {
        let mut query = Query::<&C1>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity).unwrap();
        assert_eq!(*res, C1(001));

        let mut query = Query::<&C2>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity).unwrap();
        assert_eq!(*res, C2(002));

        let mut query = Query::<&C3>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity).unwrap();
        assert_eq!(*res, C3(003));
    }

    scene.remove_components::<C1>(&entity);

    {
        let mut query = Query::<&C1>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity);
        assert!(res.is_none());

        let mut query = Query::<&C2>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity).unwrap();
        assert_eq!(*res, C2(002));

        let mut query = Query::<&C3>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity).unwrap();
        assert_eq!(*res, C3(003));
    }

    scene.remove_components::<C2>(&entity);

    {
        let mut query = Query::<&C1>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity);
        assert!(res.is_none());

        let mut query = Query::<&C2>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity);
        assert!(res.is_none());

        let mut query = Query::<&C3>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity).unwrap();
        assert_eq!(*res, C3(003));
    }

    scene.remove_components::<C3>(&entity);

    {
        let mut query = Query::<&C1>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity);
        assert!(res.is_none());

        let mut query = Query::<&C2>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity);
        assert!(res.is_none());

        let mut query = Query::<&C3>::new(&scene).unwrap();
        let res = query.get_entity_components(&entity);
        assert!(res.is_none());
    }
}

#[test]
fn test_query_iter_single_table() {
    let mut world = World::new();

    let scene = world.current_scene_mut();
    let mut ents = Vec::with_capacity(100);

    for i in 0..100 {
        let entity = scene.spawn_entity();
        scene.add_component(&entity, (C1(i), C2(i + 100)));
        ents.push(entity);
    }

    let mut query = Query::<(&C1, &mut C2)>::new(&scene).unwrap();
    assert_eq!(query.table_count(), 1);

    let iter = query.iter();

    for (i, (c1, c2)) in iter.enumerate() {
        assert_eq!(c1.0, i as u32);
        assert_eq!(c2.0, i as u32 + 100);
    }
}

#[test]
fn test_query_iter_multiple_table() {
    let mut world = World::new();

    let scene = world.current_scene_mut();
    let mut ents = Vec::with_capacity(100);

    // spawn entities
    for _ in 0..100 {
        let entity = scene.spawn_entity();
        ents.push(entity);
    }

    // add single component
    for (i, entity) in ents[0..10].iter().enumerate() {
        scene.add_component(entity, C1(i as u32));
    }

    // add double component
    for (i, entity) in ents.iter().enumerate() {
        scene.add_component(entity, C2(i as u32 + 100));
    }

    let mut query = Query::<&C1>::new(&scene).unwrap();
    assert_eq!(query.table_count(), 2);

    let mut iter = query.iter().enumerate();

    for (i, c1) in (&mut iter).take(10) {
        assert_eq!(c1.0, i as u32);
    }

    for (i, c1) in (&mut iter).take(90) {
        assert_eq!(c1.0, i as u32 + 100);
    }

    assert_eq!(iter.next(), None);
}
