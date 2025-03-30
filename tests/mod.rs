use eonix::{Component, Query, World};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct C1(u32);
impl Component for C1 {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct C2(u32);
impl Component for C2 {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct C3(u32);
impl Component for C3 {}

#[test]
fn test_query_get() {
    let mut world = World::new();

    let scene = world.current_scene_mut();
    let mut ents = Vec::with_capacity(100);

    for i in 0..100 {
        let entity = scene.spawn_entity();
        scene.add_component(entity, (C1(i), C2(i + 100)));
        ents.push(entity);
    }

    let mut query = Query::<(&C1, &mut C2)>::new(&scene).unwrap();
    assert_eq!(query.table_count(), 1);

    for (i, ent) in ents.iter().enumerate() {
        let (c1, c2) = query.get(ent).unwrap();
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

    scene.add_component(entity, (C1(42), C2(123)));

    {
        let mut query = Query::<&C1>::new(&scene).unwrap();
        let res = query.get(&entity).unwrap();
        assert_eq!(*res, C1(42));

        let mut query = Query::<&mut C2>::new(&scene).unwrap();
        let res = query.get(&entity).unwrap();
        assert_eq!(*res, C2(123));
    }

    //

    scene.add_component(entity, C1(1002));

    {
        let mut query = Query::<&mut C1>::new(&scene).unwrap();
        let res = query.get(&entity).unwrap();
        assert_eq!(*res, C1(1002));

        let mut query = Query::<&C2>::new(&scene).unwrap();
        let res = query.get(&entity).unwrap();
        assert_eq!(*res, C2(123));
    }

    //

    scene.add_component(entity, C3(090));

    {
        let mut query = Query::<&C1>::new(&scene).unwrap();
        let res = query.get(&entity).unwrap();
        assert_eq!(*res, C1(1002));

        let mut query = Query::<&C2>::new(&scene).unwrap();
        let res = query.get(&entity).unwrap();
        assert_eq!(*res, C2(123));

        let mut query = Query::<&mut C3>::new(&scene).unwrap();
        let res = query.get(&entity).unwrap();
        assert_eq!(*res, C3(090));
    }
}

#[test]
fn test_remove_components() {
    let mut world = World::new();

    let scene = world.current_scene_mut();
    let entity = scene.spawn_entity();
    scene.add_component(entity, C1(001));
    scene.add_component(entity, C2(002));
    scene.add_component(entity, C3(003));

    {
        let mut query = Query::<&C1>::new(&scene).unwrap();
        let res = query.get(&entity).unwrap();
        assert_eq!(*res, C1(001));

        let mut query = Query::<&C2>::new(&scene).unwrap();
        let res = query.get(&entity).unwrap();
        assert_eq!(*res, C2(002));

        let mut query = Query::<&C3>::new(&scene).unwrap();
        let res = query.get(&entity).unwrap();
        assert_eq!(*res, C3(003));
    }

    scene.remove_components::<C1>(&entity);

    {
        let mut query = Query::<&C1>::new(&scene).unwrap();
        let res = query.get(&entity);
        assert!(res.is_none());

        let mut query = Query::<&C2>::new(&scene).unwrap();
        let res = query.get(&entity).unwrap();
        assert_eq!(*res, C2(002));

        let mut query = Query::<&C3>::new(&scene).unwrap();
        let res = query.get(&entity).unwrap();
        assert_eq!(*res, C3(003));
    }

    scene.remove_components::<C2>(&entity);

    {
        let mut query = Query::<&C1>::new(&scene).unwrap();
        let res = query.get(&entity);
        assert!(res.is_none());

        let mut query = Query::<&C2>::new(&scene).unwrap();
        let res = query.get(&entity);
        assert!(res.is_none());

        let mut query = Query::<&C3>::new(&scene).unwrap();
        let res = query.get(&entity).unwrap();
        assert_eq!(*res, C3(003));
    }

    scene.remove_components::<C3>(&entity);

    {
        let mut query = Query::<&C1>::new(&scene).unwrap();
        let res = query.get(&entity);
        assert!(res.is_none());

        let mut query = Query::<&C2>::new(&scene).unwrap();
        let res = query.get(&entity);
        assert!(res.is_none());

        let mut query = Query::<&C3>::new(&scene).unwrap();
        let res = query.get(&entity);
        assert!(res.is_none());
    }
}

#[test]
fn test_query_iter() {
    let mut world = World::new();

    let scene = world.current_scene_mut();
    let mut ents = Vec::with_capacity(100);

    for i in 0..100 {
        let entity = scene.spawn_entity();
        scene.add_component(entity, (C1(i), C2(i + 100)));
        ents.push(entity);
    }

    let mut query = Query::<(&C1, &mut C2)>::new(&scene).unwrap();
    assert_eq!(query.table_count(), 1);

    let iter = query.iter();

    for x in iter {}
}
