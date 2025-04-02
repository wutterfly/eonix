use std::{any::TypeId, marker::PhantomData};

use crate::{
    Scene,
    components::EntityComponents,
    entity::{Entity, Generation},
    filter::Filter,
    macros::unwrap,
    system::ParamType,
    table::{Table, TableId},
};

pub struct Query<'a, E: Extract, F: Filter = ()> {
    pub tables: Vec<E::Extracted<'a>>,
    entities: &'a [(Generation, TableId)],
    _f: PhantomData<F>,
}

impl<'a, E: Extract, F: Filter> Query<'a, E, F> {
    #[inline]
    pub fn new(scene: &'a Scene) -> Option<Self> {
        let entitie_components = &scene.entities;

        Self::new_internal(entitie_components)
    }

    pub(crate) fn new_internal(entitie_components: &'a EntityComponents) -> Option<Self> {
        #[cfg(feature = "runtime-checks")]
        Self::validate();

        let extracted_tables = Self::extract_tables(&entitie_components.tables)?;

        debug_assert!(!extracted_tables.is_empty());

        Some(Self {
            tables: extracted_tables,
            entities: &entitie_components.entities,
            _f: PhantomData,
        })
    }

    #[inline]
    fn extract_tables(tables: &'a [Table]) -> Option<Vec<E::Extracted<'a>>> {
        if tables.is_empty() {
            return None;
        }

        let mut out = Vec::with_capacity(tables.len());
        for table in tables {
            if table.is_empty() || !F::check(table) {
                continue;
            }

            if let Ok(access) = E::extract(table) {
                out.push(access);
            }
        }

        if out.is_empty() {
            return None;
        }

        Some(out)
    }

    #[cfg(feature = "runtime-checks")]
    fn validate() {
        E::validate();
        F::validate();

        let e_types = E::types();
        let f_types = F::types();

        for e_t in e_types.iter() {
            for f_t in f_types.iter() {
                if e_t.raw_type() == f_t.raw_type() {
                    panic!(
                        "Extract and Filter conflict: Extract: [{}]  <-> [{}] :Filter",
                        e_t.name(),
                        f_t.name()
                    );
                }
            }
        }
    }

    #[inline]
    pub const fn table_count(&self) -> usize {
        self.tables.len()
    }

    pub fn get_entity_components(
        &mut self,
        entity: &Entity,
    ) -> Option<<E::Extracted<'a> as GetComponentAccess>::Item<'_>> {
        let (generation, table_id) = self.entities.get(entity.id())?;

        if *generation != entity.generation() {
            return None;
        }

        if table_id.is_invalid() {
            return None;
        }

        let table = self
            .tables
            .iter_mut()
            .find(|table| table.table_id() == *table_id)?;

        table.get_entity(entity)
    }

    pub fn iter(&mut self) -> QueryIter<'a, '_, E> {
        let mut iter = self.tables.iter_mut();
        let current = unwrap!(iter.next()).iter();

        QueryIter::<'_, '_, E> {
            tables: iter,
            current_table: current,
        }
    }
}

pub struct TableAccess<'a, Rows: RowAccess> {
    pub(crate) table_id: TableId,
    pub(crate) entities: &'a [Entity],
    pub(crate) table_rows: Rows,
}

pub struct QueryIter<'a, 'b, E: Extract> {
    tables: std::slice::IterMut<'b, <E as Extract>::Extracted<'a>>,
    current_table: <E::Extracted<'a> as GetComponentAccess>::Iter<'b>,
}

impl<'a, 'b, E: Extract> Iterator for QueryIter<'a, 'b, E> {
    type Item = <<E::Extracted<'a> as GetComponentAccess>::Iter<'b> as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // try get next item from current iterator
            let next_item = self.current_table.next();

            match next_item {
                // return item
                Some(item) => return Some(item),

                // table is finished
                None => {
                    // get next table
                    let next_table = self.tables.next();

                    match next_table {
                        // found next table, loop
                        Some(table) => {
                            self.current_table = table.iter();
                            continue;
                        }
                        // no more tables, all finished
                        None => return None,
                    }
                }
            }
        }
    }
}

pub enum NoneIter<I: Iterator> {
    Iter(I),
    None,
}

impl<I: Iterator> Iterator for NoneIter<I> {
    type Item = Option<I::Item>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Iter(iter_mut) => Some(iter_mut.next()),
            Self::None => Some(None),
        }
    }
}

pub trait Extract {
    type Extracted<'new>: GetComponentAccess;

    type RowOnly<'new>: RowAccess;

    #[inline]
    fn raw_unit_type() -> (TypeId, bool) {
        unimplemented!()
    }

    fn types() -> Vec<ParamType>;

    #[cfg(feature = "runtime-checks")]
    fn validate();

    fn extract(table: &'_ Table) -> Result<Self::Extracted<'_>, ()>;

    #[inline]
    fn get_row_only(_: &'_ Table) -> Result<Self::RowOnly<'_>, ()> {
        unimplemented!()
    }
}

pub trait GetComponentAccess {
    type Item<'a>
    where
        Self: 'a;

    type Iter<'a>: Iterator<Item = Self::Item<'a>>
    where
        Self: 'a;

    fn table_id(&self) -> TableId;

    fn get_entity(&mut self, entity: &Entity) -> Option<Self::Item<'_>>;

    fn iter(&mut self) -> Self::Iter<'_>;
}

pub trait RowAccess {
    type Item<'a>
    where
        Self: 'a;

    fn get_entity_components(&mut self, position: usize) -> Self::Item<'_>;

    type Iter<'a>: Iterator<Item = Self::Item<'a>>
    where
        Self: 'a;

    fn get_iter(&mut self) -> Self::Iter<'_>;
}

#[cfg(feature = "runtime-checks")]
#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use crate::Scene;

    use super::Query;

    #[test]
    fn test_invalid_same_type() {
        let scene = Scene::new();

        let res = catch_unwind(AssertUnwindSafe(|| {
            let _ = Query::<(&u32, &u32)>::new(&scene);
        }));

        assert!(res.is_err());
    }

    #[test]
    fn test_invalid_only_optional() {
        let scene = Scene::new();

        let res = catch_unwind(AssertUnwindSafe(|| {
            let _ = Query::<Option<&u32>>::new(&scene);
        }));
        assert!(res.is_err());

        let res = catch_unwind(AssertUnwindSafe(|| {
            let _ = Query::<(Option<&u32>, Option<&i32>)>::new(&scene);
        }));
        assert!(res.is_err());
    }

    #[test]
    fn test_invalid_extract_and_filter() {
        use crate::WithOut;

        let scene = Scene::new();

        let res = catch_unwind(AssertUnwindSafe(|| {
            let _ = Query::<&u32, WithOut<u32>>::new(&scene);
        }));

        assert!(res.is_err());
    }
}
