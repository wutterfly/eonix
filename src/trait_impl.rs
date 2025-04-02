use std::{
    any::TypeId,
    iter::Zip,
    ops::{Deref, DerefMut},
};

use crate::{
    Commands, Component, NoSend, Query, Resource, World,
    cells::{WorldCellComplete, WorldCellSend},
    components::ComponentSet,
    entity::Entity,
    filter::{Filter, FilterType},
    macros::{
        component_set_impl, extract_impl, filter_impl, row_access_impl, system_impl,
        table_ident_impl, unwrap,
    },
    query::{Extract, GetComponentAccess, NoneIter, RowAccess, TableAccess},
    resources::{
        GlobalRes, GlobalResMut, GlobalUnsendMut, GlobalUnsendRef, Res, ResMut, UnsendMut,
        UnsendRef,
    },
    system::{FunctionSystem, IntoSystem, ParamType, System, SystemParam},
    table::{Row, RowAccessMut, RowAccessRef, Table, TableId, TableIdBuilder, TableIdent},
    world::SendWorld,
};

// ComponentSet
const _: () = {
    impl<A: Component> ComponentSet for A {
        #[inline]
        fn types() -> Vec<TypeId> {
            vec![TypeId::of::<A>()]
        }

        #[inline]
        fn contains_type(type_id: TypeId) -> bool {
            type_id == TypeId::of::<A>()
        }

        fn push_to_table(self, table: &mut Table, entity: Entity)
        where
            Self: Sized,
        {
            debug_assert_eq!(table.rows.len(), 1);
            debug_assert_eq!(table.rows[0].tid(), TypeId::of::<A>());

            table.rows[0].push(self);
            table.entities.push(entity);
        }

        fn update_rows(self, table: &mut Table, position: usize) {
            let a = self;

            unwrap!(table.rows.iter_mut().find(|x| x.tid() == TypeId::of::<A>()))
                .update::<A>(position, a);
        }

        fn push_or_update(self, table: &mut Table, position: usize) {
            let a = self;

            unwrap!(table.rows.iter_mut().find(|x| x.tid() == TypeId::of::<A>()))
                .push_or_update::<A>(position, a);
        }
    }

    component_set_impl!(A, B);
    component_set_impl!(A, B, C);
    component_set_impl!(A, B, C, D);
    component_set_impl!(A, B, C, D, E);
    component_set_impl!(A, B, C, D, E, F);

    #[cfg(feature = "large_tuples")]
    {
        component_set_impl!(A, B, C, D, E, F, G);
        component_set_impl!(A, B, C, D, E, F, G, H);
        component_set_impl!(A, B, C, D, E, F, G, H, I);
    }
};

// TableIdent
const _: () = {
    table_ident_impl!(A);
    table_ident_impl!(A, B);
    table_ident_impl!(A, B, C);
    table_ident_impl!(A, B, C, D);
    table_ident_impl!(A, B, C, D, E);
    table_ident_impl!(A, B, C, D, E, F);

    #[cfg(feature = "large_tuples")]
    {
        table_ident_impl!(A, B, C, D, E, F, G);
        table_ident_impl!(A, B, C, D, E, F, G, H);
        table_ident_impl!(A, B, C, D, E, F, G, H, I);
    }
};

// Extract
const _: () = {
    impl<C: Component> Extract for &C {
        type Extracted<'new> = TableAccess<'new, Self::RowOnly<'new>>;
        type RowOnly<'new> = RowAccessRef<'new, C>;

        #[inline]
        fn raw_unit_type() -> (TypeId, bool) {
            (TypeId::of::<C>(), true)
        }

        #[inline]
        fn types() -> Vec<ParamType> {
            vec![ParamType::new_shared::<C>()]
        }

        #[cfg(feature = "runtime-checks")]
        fn validate() {}

        #[inline]
        fn extract(table: &'_ Table) -> Result<Self::Extracted<'_>, ()> {
            let entities = &table.entities;

            let access = TableAccess {
                table_id: table.id(),
                entities,
                table_rows: table.try_get_row_ref::<C>()?,
            };

            Ok(access)
        }

        #[inline]
        fn get_row_only(table: &'_ Table) -> Result<Self::RowOnly<'_>, ()> {
            table.try_get_row_ref()
        }
    }

    impl<C: Component> Extract for &mut C {
        type Extracted<'new> = TableAccess<'new, Self::RowOnly<'new>>;
        type RowOnly<'new> = RowAccessMut<'new, C>;

        #[inline]
        fn raw_unit_type() -> (TypeId, bool) {
            (TypeId::of::<C>(), true)
        }

        #[inline]
        fn types() -> Vec<ParamType> {
            vec![ParamType::new_mut::<C>()]
        }

        #[cfg(feature = "runtime-checks")]
        fn validate() {}

        #[inline]
        fn extract(table: &'_ Table) -> Result<Self::Extracted<'_>, ()> {
            let entities = &table.entities;

            let access = TableAccess {
                table_id: table.id(),
                entities,
                table_rows: table.try_get_row_mut::<C>()?,
            };

            Ok(access)
        }

        #[inline]
        fn get_row_only(table: &'_ Table) -> Result<Self::RowOnly<'_>, ()> {
            table.try_get_row_mut()
        }
    }

    impl<C: Component> Extract for Option<&C> {
        type Extracted<'new> = TableAccess<'new, Self::RowOnly<'new>>;
        type RowOnly<'new> = Option<RowAccessRef<'new, C>>;

        #[inline]
        fn raw_unit_type() -> (TypeId, bool) {
            (TypeId::of::<C>(), false)
        }

        #[inline]
        fn types() -> Vec<ParamType> {
            vec![ParamType::new_shared::<C>()]
        }

        #[cfg(feature = "runtime-checks")]
        fn validate() {
            panic!("Only Option<&C> is not a valid input for queries!")
        }

        #[inline]
        fn extract(table: &'_ Table) -> Result<Self::Extracted<'_>, ()> {
            let entities = &table.entities;

            let access = TableAccess {
                table_id: table.id(),
                entities,
                table_rows: Self::get_row_only(table)?,
            };

            Ok(access)
        }

        #[inline]
        fn get_row_only(table: &'_ Table) -> Result<Self::RowOnly<'_>, ()> {
            Ok(table.try_get_row_ref().ok())
        }
    }

    impl<C: Component> Extract for Option<&mut C> {
        type Extracted<'new> = TableAccess<'new, Self::RowOnly<'new>>;
        type RowOnly<'new> = Option<RowAccessMut<'new, C>>;

        #[inline]
        fn raw_unit_type() -> (TypeId, bool) {
            (TypeId::of::<C>(), false)
        }

        #[inline]
        fn types() -> Vec<ParamType> {
            vec![ParamType::new_mut::<C>()]
        }

        #[cfg(feature = "runtime-checks")]
        fn validate() {
            panic!("Only Option<&mut C> is not a valid input for queries!")
        }

        #[inline]
        fn extract(table: &'_ Table) -> Result<Self::Extracted<'_>, ()> {
            let entities = &table.entities;

            let access = TableAccess {
                table_id: table.id(),
                entities,
                table_rows: Self::get_row_only(table)?,
            };

            Ok(access)
        }

        #[inline]
        fn get_row_only(table: &'_ Table) -> Result<Self::RowOnly<'_>, ()> {
            Ok(table.try_get_row_mut().ok())
        }
    }

    extract_impl!(A, B);
    extract_impl!(A, B, C);
    extract_impl!(A, B, C, D);
    extract_impl!(A, B, C, D, E);
    extract_impl!(A, B, C, D, E, F);

    #[cfg(feature = "large_tuples")]
    {
        extract_impl!(A, B, C, D, E, F, G);
        extract_impl!(A, B, C, D, E, F, G, H);
        extract_impl!(A, B, C, D, E, F, G, H, I);
    }
};

// GetComponentAccess
const _: () = {
    impl<A: RowAccess> GetComponentAccess for TableAccess<'_, A> {
        type Item<'a>
            = A::Item<'a>
        where
            Self: 'a;

        type Iter<'a>
            = A::Iter<'a>
        where
            Self: 'a;

        #[inline]
        fn table_id(&self) -> TableId {
            self.table_id
        }

        #[inline]
        fn get_entity(&mut self, entity: &Entity) -> Option<Self::Item<'_>> {
            let position = self.entities.iter().position(|ent| ent == entity)?;

            Some(self.table_rows.get_entity_components(position))
        }

        #[inline]
        fn iter(&mut self) -> Self::Iter<'_> {
            self.table_rows.get_iter()
        }
    }
};

// RowAccess
const _: () = {
    impl<C: Component> RowAccess for RowAccessRef<'_, C> {
        type Item<'a>
            = &'a C
        where
            Self: 'a;

        #[inline]
        fn get_entity_components(&mut self, position: usize) -> Self::Item<'_> {
            unwrap!(RowAccessRef::deref(self).get(position))
        }

        type Iter<'a>
            = std::slice::Iter<'a, C>
        where
            Self: 'a;

        #[inline]
        fn get_iter(&mut self) -> Self::Iter<'_> {
            RowAccessRef::deref(self).iter()
        }
    }

    impl<C: Component> RowAccess for RowAccessMut<'_, C> {
        type Item<'new>
            = &'new mut C
        where
            Self: 'new;

        #[inline]
        fn get_entity_components(&mut self, position: usize) -> Self::Item<'_> {
            unwrap!(RowAccessMut::deref_mut(self).get_mut(position))
        }

        type Iter<'a>
            = std::slice::IterMut<'a, C>
        where
            Self: 'a;

        #[inline]
        fn get_iter(&mut self) -> Self::Iter<'_> {
            RowAccessMut::deref_mut(self).iter_mut()
        }
    }

    impl<C: Component> RowAccess for Option<RowAccessRef<'_, C>> {
        type Item<'a>
            = Option<&'a C>
        where
            Self: 'a;

        #[inline]
        fn get_entity_components(&mut self, position: usize) -> Self::Item<'_> {
            let out = self.as_mut()?.get(position);

            #[cfg(feature = "runtime-checks")]
            assert!(out.is_some());

            out
        }

        type Iter<'a>
            = NoneIter<std::slice::Iter<'a, C>>
        where
            Self: 'a;

        #[inline]
        fn get_iter(&mut self) -> Self::Iter<'_> {
            self.as_mut()
                .map_or(NoneIter::None, |row| NoneIter::Iter(row.get_iter()))
        }
    }

    impl<C: Component> RowAccess for Option<RowAccessMut<'_, C>> {
        type Item<'new>
            = Option<&'new mut C>
        where
            Self: 'new;

        #[inline]
        fn get_entity_components(&mut self, position: usize) -> Self::Item<'_> {
            let out = self.as_mut()?.get_mut(position);

            #[cfg(feature = "runtime-checks")]
            assert!(out.is_some());

            out
        }

        type Iter<'a>
            = NoneIter<std::slice::IterMut<'a, C>>
        where
            Self: 'a;

        #[inline]
        fn get_iter(&mut self) -> Self::Iter<'_> {
            self.as_mut()
                .map_or(NoneIter::None, |row| NoneIter::Iter(row.get_iter()))
        }
    }

    impl<A: RowAccess, B: RowAccess> RowAccess for (A, B) {
        type Item<'a>
            = (A::Item<'a>, B::Item<'a>)
        where
            Self: 'a;

        #[inline]
        fn get_entity_components(&mut self, position: usize) -> Self::Item<'_> {
            let (a, b) = self;

            (
                a.get_entity_components(position),
                b.get_entity_components(position),
            )
        }

        type Iter<'a>
            = Zip<A::Iter<'a>, B::Iter<'a>>
        where
            A: 'a,
            B: 'a;

        #[inline]
        fn get_iter(&mut self) -> Self::Iter<'_> {
            let (a, b) = self;

            a.get_iter().zip(b.get_iter())
        }
    }

    row_access_impl!(A, B, C);
    row_access_impl!(A, B, C, D);
    row_access_impl!(A, B, C, D, E);
    row_access_impl!(A, B, C, D, E, F);

    #[cfg(feature = "large_tuples")]
    {
        row_access_impl!(A, B, C, D, E, F, G);
        row_access_impl!(A, B, C, D, E, F, G, H);
        row_access_impl!(A, B, C, D, E, F, G, H, I);
    }
};

// SystemParam
const _: () = {
    impl<R: Resource> SystemParam for Res<'_, R> {
        type Item<'new> = Res<'new, R>;

        #[inline]
        fn get_types() -> Vec<ParamType> {
            vec![ParamType::new_shared::<R>()]
        }

        #[inline]
        fn retrieve(world: SendWorld) -> Option<Self::Item<'_>> {
            world.scene.get_resource_ref()
        }
    }

    impl<R: Resource> SystemParam for ResMut<'_, R> {
        type Item<'new> = ResMut<'new, R>;

        #[inline]
        fn get_types() -> Vec<ParamType> {
            vec![ParamType::new_mut::<R>()]
        }

        #[inline]
        fn retrieve(world: SendWorld) -> Option<Self::Item<'_>> {
            world.scene.get_resource_mut()
        }
    }

    impl<R: Resource> SystemParam for GlobalRes<'_, R> {
        type Item<'new> = GlobalRes<'new, R>;

        #[inline]
        fn get_types() -> Vec<ParamType> {
            vec![ParamType::new_shared::<R>()]
        }

        #[inline]
        fn retrieve(world: SendWorld) -> Option<Self::Item<'_>> {
            world
                .global_resource
                .get_resource_ref::<R>()
                .map(Into::into)
        }
    }

    impl<R: Resource> SystemParam for GlobalResMut<'_, R> {
        type Item<'new> = GlobalResMut<'new, R>;

        #[inline]
        fn get_types() -> Vec<ParamType> {
            vec![ParamType::new_mut::<R>()]
        }

        #[inline]
        fn retrieve(world: SendWorld) -> Option<Self::Item<'_>> {
            world
                .global_resource
                .get_resource_mut::<R>()
                .map(Into::into)
        }
    }

    impl<R: NoSend> SystemParam for UnsendRef<'_, R> {
        type Item<'new> = UnsendRef<'new, R>;

        fn get_types() -> Vec<ParamType> {
            vec![ParamType::new_shared::<R>()]
        }

        #[inline]
        fn local() -> bool {
            true
        }

        #[inline]
        fn retrieve(_: SendWorld) -> Option<Self::Item<'_>> {
            unimplemented!()
        }

        fn retrieve_local(world: &World) -> Option<Self::Item<'_>> {
            world
                .current_scene()
                .unsend
                .get_resource_ref::<R>()
                .map(Into::into)
        }
    }

    impl<R: NoSend> SystemParam for UnsendMut<'_, R> {
        type Item<'new> = UnsendMut<'new, R>;

        #[inline]
        fn get_types() -> Vec<ParamType> {
            vec![ParamType::new_mut::<R>()]
        }

        #[inline]
        fn local() -> bool {
            true
        }

        #[inline]
        fn retrieve(_: SendWorld) -> Option<Self::Item<'_>> {
            unimplemented!()
        }

        fn retrieve_local(world: &World) -> Option<Self::Item<'_>> {
            world
                .current_scene()
                .unsend
                .get_resource_mut::<R>()
                .map(Into::into)
        }
    }

    impl<R: NoSend> SystemParam for GlobalUnsendRef<'_, R> {
        type Item<'new> = GlobalUnsendRef<'new, R>;

        fn get_types() -> Vec<ParamType> {
            vec![ParamType::new_shared::<R>()]
        }

        #[inline]
        fn local() -> bool {
            true
        }

        fn retrieve(_: SendWorld) -> Option<Self::Item<'_>> {
            unimplemented!()
        }

        fn retrieve_local(world: &World) -> Option<Self::Item<'_>> {
            world
                .global_nosend()
                .get_resource_ref::<R>()
                .map(Into::into)
        }
    }

    impl<R: NoSend> SystemParam for GlobalUnsendMut<'_, R> {
        type Item<'new> = GlobalUnsendMut<'new, R>;

        fn get_types() -> Vec<ParamType> {
            vec![ParamType::new_mut::<R>()]
        }

        #[inline]
        fn local() -> bool {
            true
        }

        #[inline]
        fn retrieve(_: SendWorld) -> Option<Self::Item<'_>> {
            unimplemented!()
        }

        fn retrieve_local(world: &World) -> Option<Self::Item<'_>> {
            world
                .global_nosend()
                .get_resource_mut::<R>()
                .map(Into::into)
        }
    }

    impl SystemParam for Commands {
        type Item<'new> = Self;

        #[inline]
        fn get_types() -> Vec<ParamType> {
            vec![ParamType::new_shared::<Self>()]
        }

        #[inline]
        fn retrieve(world: SendWorld) -> Option<Self::Item<'_>> {
            Some(world.commands.commands(world.scene.entities.spawner()))
        }
    }

    impl<E: Extract> SystemParam for Query<'_, E> {
        type Item<'new> = Query<'new, E>;

        #[inline]
        fn get_types() -> Vec<ParamType> {
            E::types()
        }

        #[inline]
        fn retrieve(world: SendWorld) -> Option<Self::Item<'_>> {
            Query::new_internal(world.scene.entities)
        }
    }
};

// IntoSystem & System
const _: () = {
    impl<F: Fn() + Send + Sync> System for FunctionSystem<(), F> {
        #[inline]
        fn get_types(&self) -> Vec<ParamType> {
            vec![ParamType::new_shared::<()>()]
        }

        #[inline]
        fn local(&self) -> bool {
            false
        }

        #[cfg(feature = "debug-utils")]
        #[inline]
        fn name(&self) -> &'static str {
            std::any::type_name::<F>()
        }

        #[inline]
        fn run(&self, _: WorldCellSend) -> Result<(), ()> {
            (self.f)();
            Ok(())
        }

        #[inline]
        fn run_on_main(&self, _: WorldCellComplete) -> Result<(), ()> {
            unimplemented!()
        }
    }

    impl<F: Fn() + Send + Sync> IntoSystem<()> for F {
        type System = FunctionSystem<(), Self>;

        #[inline]
        fn into_system(self) -> Self::System {
            FunctionSystem {
                f: self,
                marker: Default::default(),
            }
        }
    }

    impl<'x, FF: Fn(&'x mut World) + Send + Sync> IntoSystem<&'x mut World> for FF
    where
        for<'a, 'b> &'a FF: Fn(&mut World) + Fn(&mut World),
    {
        type System = FunctionSystem<&'x mut World, Self>;

        fn into_system(self) -> Self::System {
            FunctionSystem {
                f: self,
                marker: Default::default(),
            }
        }
    }

    impl<FF> System for FunctionSystem<&mut World, FF>
    where
        for<'a, 'b> &'a FF: Fn(&mut World) + Fn(&mut World),
        FF: Send + Sync,
    {
        #[inline]
        fn get_types(&self) -> Vec<ParamType> {
            vec![ParamType::World]
        }

        #[inline]
        fn local(&self) -> bool {
            true
        }

        #[cfg(feature = "debug-utils")]
        fn name(&self) -> &'static str {
            std::any::type_name::<FF>()
        }

        #[inline]
        fn run(&self, _: WorldCellSend) -> Result<(), ()> {
            unimplemented!()
        }

        fn run_on_main(&self, world: WorldCellComplete) -> Result<(), ()> {
            fn call_inner(f: impl Fn(&mut World), world: &mut World) {
                f(world)
            }

            call_inner(&self.f, *world.borrow_mut());
            Ok(())
        }
    }

    system_impl!(A);
    system_impl!(A, B);
    system_impl!(A, B, C);
};

// Filter
const _: () = {
    impl Filter for () {
        #[inline]
        fn types() -> Vec<FilterType> {
            vec![]
        }

        #[cfg(feature = "runtime-checks")]
        fn validate() {}

        #[inline]
        fn check(_: &Table) -> bool {
            true
        }
    }

    filter_impl!(F1, F2);
    filter_impl!(F1, F2, F3);
    filter_impl!(F1, F2, F3, F4);
    filter_impl!(F1, F2, F3, F4, F5);
    filter_impl!(F1, F2, F3, F4, F5, F6);
};

#[cfg(feature = "runtime-checks")]
fn unique_tuple<const N: usize>(types: &[TypeId; N]) {
    for (i, t1) in types.iter().enumerate() {
        for (j, t2) in types.iter().enumerate() {
            if i != j {
                assert_ne!(t1, t2)
            }
        }
    }
}
