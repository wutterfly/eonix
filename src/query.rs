use crate::{
    Scene,
    entity::{Entity, Generation},
    table::{Table, TableId},
};

pub struct Query<'a, E: Extract> {
    pub tables: Vec<E::Extracted<'a>>,
    entities: &'a [(Generation, TableId)],
}

impl<'a, E: Extract> Query<'a, E> {
    #[inline]
    pub fn new(scene: &'a Scene) -> Result<Self, ()> {
        let tables = &scene.entities.tables;
        let mut out = Vec::with_capacity(tables.len());
        for table in tables {
            if let Ok(access) = E::extract(table) {
                out.push(access);
            }
        }

        if tables.is_empty() {
            return Err(());
        }

        Ok(Self {
            tables: out,
            entities: &scene.entities.entities,
        })
    }

    #[inline]
    pub const fn table_count(&self) -> usize {
        self.tables.len()
    }

    pub fn get(
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

    pub fn iter(
        &mut self,
    ) -> impl Iterator<Item = <<E as Extract>::Extracted<'a> as GetComponentAccess>::Item<'_>> {
        let mut iter = self.tables.iter_mut();
        let current = iter.next().unwrap().iter();

        QueryIter::<'_, '_, E, _> {
            table: 0,
            ent: 0,
            tables: iter,
            current,
        }
    }
}

pub struct QueryIter<'a, 'b, E: Extract, I>
where
    I: Iterator<Item = <<E as Extract>::Extracted<'a> as GetComponentAccess>::Item<'b>>,
{
    table: usize,
    ent: usize,
    tables: std::slice::IterMut<'b, <E as Extract>::Extracted<'a>>,
    current: I,
}

impl<'a, 'b, E: Extract, I> Iterator for QueryIter<'a, 'b, E, I>
where
    I: Iterator<Item = <<E as Extract>::Extracted<'a> as GetComponentAccess>::Item<'b>>,
{
    type Item = <<E as Extract>::Extracted<'a> as GetComponentAccess>::Item<'b>;

    fn next(&mut self) -> Option<Self::Item> {
        let x = self.current.next();
        x
    }
}

pub struct TableAccess<'a, A> {
    pub(crate) table_id: TableId,
    pub(crate) entities: &'a [Entity],
    pub(crate) extracted: A,
}

impl<A: RowAccess> TableAccess<'_, A> {
    pub fn get_iter(&mut self) -> impl Iterator<Item = A::Item<'_>> {
        TableAccessIter {
            extracted: &mut self.extracted,
        }
    }
}

struct TableAccessIter<'a, A> {
    extracted: &'a mut A,
}

impl<'a, A: RowAccess + 'a> Iterator for TableAccessIter<'a, A> {
    type Item = A::Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

pub trait Extract {
    type Extracted<'new>: GetComponentAccess;
    type RowOnly<'new>: RowAccess;

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

    fn table_id(&self) -> TableId;

    fn get_entity(&mut self, entity: &Entity) -> Option<Self::Item<'_>>;

    fn iter(&mut self) -> impl Iterator<Item = Self::Item<'_>>;
}

pub trait RowAccess {
    type Item<'new>
    where
        Self: 'new;

    fn get(&mut self, position: usize) -> Option<Self::Item<'_>>;
}
