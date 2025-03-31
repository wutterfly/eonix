use std::{
    any::TypeId,
    iter::Zip,
    ops::{Deref, DerefMut},
};

use crate::{
    Component,
    components::ComponentSet,
    entity::Entity,
    macros::{component_set_impl, extract_impl, row_access_impl, table_ident_impl, unwrap},
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

#[inline]
fn unique_tuple<const N: usize>(_types: &[TypeId; N]) {
    #[cfg(feature = "runtime-checks")]
    {
        for (i, t1) in _types.iter().enumerate() {
            for (j, t2) in _types.iter().enumerate() {
                if i != j {
                    assert_ne!(t1, t2)
                }
            }
        }
    }
}
