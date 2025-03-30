use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

use crate::{
    Component,
    components::ComponentSet,
    entity::Entity,
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

            table
                .rows
                .iter_mut()
                .find(|x| x.tid() == TypeId::of::<A>())
                .unwrap()
                .update::<A>(position, a);
        }

        fn push_or_update(self, table: &mut Table, position: usize) {
            let a = self;

            table
                .rows
                .iter_mut()
                .find(|x| x.tid() == TypeId::of::<A>())
                .unwrap()
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

            table
                .rows
                .iter_mut()
                .find(|x| x.tid() == TypeId::of::<A>())
                .unwrap()
                .push(a);

            table
                .rows
                .iter_mut()
                .find(|x| x.tid() == TypeId::of::<B>())
                .unwrap()
                .push(b);

            table.entities.push(entity);
        }

        fn update_rows(self, table: &mut Table, position: usize) {
            debug_assert!(table.rows.len() >= 2);

            let (a, b) = self;

            table
                .rows
                .iter_mut()
                .find(|x| x.tid() == TypeId::of::<A>())
                .unwrap()
                .update::<A>(position, a);

            table
                .rows
                .iter_mut()
                .find(|x| x.tid() == TypeId::of::<B>())
                .unwrap()
                .update::<B>(position, b);
        }

        fn push_or_update(self, table: &mut Table, position: usize) {
            debug_assert_eq!(table.rows.len(), 2);

            let (a, b) = self;

            table
                .rows
                .iter_mut()
                .find(|x| x.tid() == TypeId::of::<A>())
                .unwrap()
                .push_or_update::<A>(position, a);

            table
                .rows
                .iter_mut()
                .find(|x| x.tid() == TypeId::of::<B>())
                .unwrap()
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
        fn table_id() -> TableId {
            let mut builder = TableIdBuilder::new();

            builder.add_unqiue(TypeId::of::<A>());
            builder.add_unqiue(TypeId::of::<B>());

            builder.finish()
        }

        fn row_count() -> usize {
            2
        }

        fn rows() -> Box<[Row]> {
            Box::new([Row::new::<A>(), Row::new::<B>()])
        }
    }
};

// Extract
const _: () = {
    impl<C: Component> Extract for &C {
        type Extracted<'new> = TableAccess<'new, Self::RowOnly<'new>>;
        type RowOnly<'new> = RowAccessRef<'new, C>;

        #[inline]
        fn extract(table: &'_ Table) -> Result<Self::Extracted<'_>, ()> {
            let entities = &table.entities;

            let access = TableAccess {
                table_id: table.id(),
                entities,
                extracted: table.try_get_row_ref::<C>()?,
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
        fn extract(table: &'_ Table) -> Result<Self::Extracted<'_>, ()> {
            let entities = &table.entities;

            let access = TableAccess {
                table_id: table.id(),
                entities,
                extracted: table.try_get_row_mut::<C>()?,
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
        fn extract(table: &'_ Table) -> Result<Self::Extracted<'_>, ()> {
            let entities = &table.entities;

            let access = TableAccess {
                table_id: table.id(),
                entities,
                extracted: (A::get_row_only(table)?, B::get_row_only(table)?),
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

        #[inline]
        fn table_id(&self) -> TableId {
            self.table_id
        }

        #[inline]
        fn get_entity(&mut self, entity: &Entity) -> Option<Self::Item<'_>> {
            let position = self.entities.iter().position(|ent| ent == entity).unwrap();

            self.extracted.get(position)
        }

        fn iter(&mut self) -> impl Iterator<Item = Self::Item<'_>> {
            self.get_iter()
        }
    }
};

// RowAccess
const _: () = {
    impl<C: Component> RowAccess for RowAccessRef<'_, C> {
        type Item<'new>
            = &'new C
        where
            Self: 'new;

        #[inline]
        fn get(&mut self, position: usize) -> Option<Self::Item<'_>> {
            RowAccessRef::deref(self).get(position)
        }
    }

    impl<C: Component> RowAccess for RowAccessMut<'_, C> {
        type Item<'new>
            = &'new mut C
        where
            Self: 'new;

        #[inline]
        fn get(&mut self, position: usize) -> Option<Self::Item<'_>> {
            RowAccessMut::deref_mut(self).get_mut(position)
        }
    }

    impl<A: RowAccess, B: RowAccess> RowAccess for (A, B) {
        type Item<'new>
            = (A::Item<'new>, B::Item<'new>)
        where
            Self: 'new;

        #[inline]
        fn get(&mut self, position: usize) -> Option<Self::Item<'_>> {
            let (a, b) = self;

            Some((a.get(position)?, b.get(position)?))
        }
    }
};
