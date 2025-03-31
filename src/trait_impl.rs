use std::{
    any::TypeId,
    iter::Zip,
    ops::{Deref, DerefMut},
};

use crate::{
    Component,
    components::ComponentSet,
    entity::Entity,
    macros::unwrap,
    query::{Extract, GetComponentAccess, RowAccess, TableAccess},
    table::{Row, RowAccessMut, RowAccessRef, Table, TableId, TableIdBuilder, TableIdent},
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

    impl<A: Component, B: Component> ComponentSet for (A, B) {
        #[inline]
        fn types() -> Vec<TypeId> {
            vec![TypeId::of::<A>(), TypeId::of::<B>()]
        }

        #[inline]
        fn contains_type(type_id: TypeId) -> bool {
            type_id == TypeId::of::<A>() || type_id == TypeId::of::<B>()
        }

        fn push_to_table(self, table: &mut Table, entity: Entity)
        where
            Self: Sized,
        {
            debug_assert_eq!(table.rows.len(), 2);

            let (a, b) = self;

            unwrap!(table.rows.iter_mut().find(|x| x.tid() == TypeId::of::<A>())).push(a);

            unwrap!(table.rows.iter_mut().find(|x| x.tid() == TypeId::of::<B>())).push(b);

            table.entities.push(entity);
        }

        fn update_rows(self, table: &mut Table, position: usize) {
            debug_assert!(table.rows.len() >= 2);

            let (a, b) = self;

            unwrap!(table.rows.iter_mut().find(|x| x.tid() == TypeId::of::<A>()))
                .update::<A>(position, a);

            unwrap!(table.rows.iter_mut().find(|x| x.tid() == TypeId::of::<B>()))
                .update::<B>(position, b);
        }

        fn push_or_update(self, table: &mut Table, position: usize) {
            debug_assert_eq!(table.rows.len(), 2);

            let (a, b) = self;

            unwrap!(table.rows.iter_mut().find(|x| x.tid() == TypeId::of::<A>()))
                .push_or_update::<A>(position, a);

            unwrap!(table.rows.iter_mut().find(|x| x.tid() == TypeId::of::<B>()))
                .push_or_update::<B>(position, b);
        }
    }
};

// TableIdent
const _: () = {
    impl<A: Component> TableIdent for A {
        fn table_id() -> TableId {
            let mut builder = TableIdBuilder::new();

            builder.add_unqiue(TypeId::of::<A>());

            builder.finish()
        }

        fn row_count() -> usize {
            1
        }

        fn rows() -> Box<[Row]> {
            Box::new([Row::new::<Self>()])
        }
    }

    impl<A: Component, B: Component> TableIdent for (A, B) {
        #[inline]
        fn validate() {
            unique_tuple(&[TypeId::of::<A>(), TypeId::of::<B>()]);
        }

        fn table_id() -> TableId {
            debug_assert!({
                let slice = [TypeId::of::<A>(), TypeId::of::<B>()];
                !(1..slice.len()).any(|i| slice[i..].contains(&slice[i - 1]))
            });

            let mut builder = TableIdBuilder::new();

            builder.add_unqiue(TypeId::of::<A>());
            builder.add_unqiue(TypeId::of::<B>());

            builder.finish()
        }

        fn row_count() -> usize {
            0 + 1 + 1
        }

        fn rows() -> Box<[Row]> {
            Box::new([Row::new::<A>(), Row::new::<B>()])
        }
    }

    impl<A: Component, B: Component, C: Component> TableIdent for (A, B, C) {
        #[inline]
        fn validate() {
            unique_tuple(&[TypeId::of::<A>(), TypeId::of::<B>(), TypeId::of::<C>()]);
        }

        fn table_id() -> TableId {
            let mut builder = TableIdBuilder::new();

            builder.add_unqiue(TypeId::of::<A>());
            builder.add_unqiue(TypeId::of::<B>());
            builder.add_unqiue(TypeId::of::<C>());

            builder.finish()
        }

        fn row_count() -> usize {
            0 + 1 + 1 + 1
        }

        fn rows() -> Box<[Row]> {
            Box::new([Row::new::<A>(), Row::new::<B>(), Row::new::<C>()])
        }
    }
};

// Extract
const _: () = {
    impl<C: Component> Extract for &C {
        type Extracted<'new> = TableAccess<'new, Self::RowOnly<'new>>;
        type RowOnly<'new> = RowAccessRef<'new, C>;

        #[inline]
        fn raw_type() -> TypeId {
            TypeId::of::<C>()
        }

        #[inline]
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
        fn raw_type() -> TypeId {
            TypeId::of::<C>()
        }

        #[inline]
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

    impl<A: Extract, B: Extract> Extract for (A, B) {
        type Extracted<'new> = TableAccess<'new, Self::RowOnly<'new>>;
        type RowOnly<'new> = (A::RowOnly<'new>, B::RowOnly<'new>);

        #[inline]
        fn validate() {
            unique_tuple(&[A::raw_type(), B::raw_type()]);
        }

        #[inline]
        fn extract(table: &'_ Table) -> Result<Self::Extracted<'_>, ()> {
            let entities = &table.entities;

            let access = TableAccess {
                table_id: table.id(),
                entities,
                table_rows: (A::get_row_only(table)?, B::get_row_only(table)?),
            };

            Ok(access)
        }
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
};

#[inline]
fn unique_tuple<const N: usize>(_types: &[TypeId; N]) {
    #[cfg(debug_assertions)]
    {
        for (i, t1) in _types.iter().enumerate() {
            for (j, t2) in _types.iter().enumerate() {
                if i != j {
                    debug_assert_ne!(t1, t2)
                }
            }
        }
    }
}
