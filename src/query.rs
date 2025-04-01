use std::any::TypeId;

use crate::{
    Scene,
    entity::{Entity, Generation},
    macros::unwrap,
    table::{Table, TableId},
};

pub struct Query<'a, E: Extract> {
    pub tables: Vec<E::Extracted<'a>>,
    entities: &'a [(Generation, TableId)],
}

impl<'a, E: Extract> Query<'a, E> {
    #[inline]
    pub fn new(scene: &'a Scene) -> Result<Self, ()> {
        E::validate();

        let extracted_tables = Self::extract_tables(&scene.entities.tables)?;

        debug_assert!(extracted_tables.len() > 0);

        Ok(Self {
            tables: extracted_tables,
            entities: &scene.entities.entities,
        })
    }

    #[inline]
    fn extract_tables(tables: &'a [Table]) -> Result<Vec<E::Extracted<'a>>, ()> {
        if tables.is_empty() {
            return Err(());
        }

        let mut out = Vec::with_capacity(tables.len());
        for table in tables {
            if table.is_empty() {
                continue;
            }

            if let Ok(access) = E::extract(table) {
                out.push(access);
            }
        }

        if out.is_empty() {
            return Err(());
        }

        Ok(out)
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
    fn raw_type() -> (TypeId, bool) {
        unimplemented!()
    }

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

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use crate::Scene;

    use super::Query;

    #[cfg(feature = "runtime-checks")]
    #[test]
    fn test_invalid_same_type() {
        let scene = Scene::new();

        let res = catch_unwind(AssertUnwindSafe(|| {
            let _ = Query::<(&u32, &u32)>::new(&scene);
        }));

        assert!(res.is_err());
    }

    #[cfg(feature = "runtime-checks")]
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
}
