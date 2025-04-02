use std::{
    any::{Any, TypeId, type_name},
    hash::{Hash, Hasher},
    marker::PhantomData,
};

//use std::hash::DefaultHasher;
use rustc_hash::FxHasher as DefaultHasher;

use crate::{
    Component,
    cells::{AtomicRefCell, MutGuard, RefGuard},
    components::ComponentSet,
    entity::Entity,
    macros::unwrap,
};

type RowComponent = dyn Any + Send + Sync + 'static;

pub trait TableIdent {
    #[inline]
    fn validate() {}

    fn table_id() -> TableId;

    fn row_count() -> usize;

    fn rows() -> Box<[Row]>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableId(u64, u64);

impl TableId {
    #[inline]
    pub const fn invalid() -> Self {
        Self(0, 0)
    }

    #[inline]
    pub const fn is_invalid(&self) -> bool {
        self.0 == 0 && self.1 == 0
    }

    pub fn from_uniques<'a>(set: impl Iterator<Item = &'a TypeId>) -> Self {
        #[cfg(feature = "runtime-checks")]
        let mut check = std::collections::HashSet::<TypeId>::new();

        let mut builder = TableIdBuilder::new();

        for id in set {
            #[cfg(feature = "runtime-checks")]
            assert!(check.insert(*id));
            builder.add_unqiue(*id);
        }

        builder.finish()
    }
}

pub struct TableIdBuilder {
    xor: u64,
    sum: u64,
    cnt: u8,
}

impl TableIdBuilder {
    const CLEAR_MASK: u64 = 0xFFFF_FFFF_FFFF_FF00;

    pub const fn new() -> Self {
        Self {
            xor: 0,
            sum: 0,
            cnt: 0,
        }
    }

    pub fn add_unqiue(&mut self, id: TypeId) {
        let mut hasher = DefaultHasher::default();
        id.hash(&mut hasher);
        let hash = hasher.finish();

        self.xor |= hash;
        self.sum = self.sum.wrapping_add(hash);
        self.cnt += 1;
    }

    pub fn finish(&self) -> TableId {
        let cnt = self.cnt as u64;

        // clear lower 8 bits and set to cnt
        let mut xor = self.xor & Self::CLEAR_MASK;
        xor |= cnt;

        debug_assert_ne!(self.cnt, 0);
        TableId(self.sum, xor)
    }
}

pub struct Table {
    id: TableId,
    // rows[0] : [  ]
    // rows[..]: [  ]
    // Entities: [  ]
    pub rows: Box<[Row]>,
    pub entities: Vec<Entity>,
}

impl Table {
    pub fn new<C: ComponentSet>() -> Self {
        C::validate();

        Self {
            id: C::table_id(),
            rows: C::rows(),
            entities: Vec::new(),
        }
    }

    pub fn get_extendable_precomputed(&self, id: TableId) -> ExtendableTable {
        ExtendableTable {
            id,
            rows: self.rows.iter().map(|row| row.clone_empty()).collect(),
            entities: Vec::new(),
        }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.entities.len()
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Updates the components of the Entity in place.
    ///
    /// Given components and table have to match!
    pub fn update<C: ComponentSet>(&mut self, entity: &Entity, components: C) {
        debug_assert_eq!(self.id, C::table_id());

        let position = self.get_entity_position(entity);

        // update components
        C::update_rows(components, self, position);
    }

    pub fn update_partial<C: ComponentSet>(&mut self, entity: &Entity, components: C) {
        let position = self.get_entity_position(entity);

        C::update_rows(components, self, position);
    }

    /// Appends Entity and components to this table.
    ///
    /// Given components and table have to match!
    pub fn push<C: ComponentSet>(&mut self, entity: Entity, components: C) {
        debug_assert_eq!(self.id, C::table_id());

        // check if entity already in table
        debug_assert!(!self.entities.contains(&entity));

        C::push_to_table(components, self, entity);
    }

    /// Removes the Entity and all its components from the table.
    pub fn delete_entity(&mut self, entity: Entity) {
        // find entity position
        let position = self.get_entity_position(&entity);
        let ent = self.entities[position];

        // remove all components
        for row in &mut self.rows {
            row.swap_remove(position);
        }

        // remove entity
        let removed = self.entities.swap_remove(position);
        debug_assert_eq!(removed, ent);
    }

    pub fn push_missing_or_update<C: ComponentSet>(&mut self, entity: &Entity, components: C) {
        let position = self.get_entity_position(entity);
        C::push_or_update(components, self, position);
    }

    /// Moves an Entity from Self to dst, for every row that self has.
    pub fn move_entity_up(&mut self, dst: &mut Self, entity: &Entity) {
        let position = self.get_entity_position(entity);

        'outer: for src_row in &mut self.rows {
            for dst_row in &mut dst.rows {
                if src_row.tid() == dst_row.tid() {
                    src_row.move_push_entity(dst_row, position);
                    continue 'outer;
                }
            }

            unreachable!("dst should have all rows that self has");
        }

        let removed = self.entities.swap_remove(position);
        debug_assert_eq!(removed, *entity);
        dst.entities.push(*entity);
    }

    /// Moves an Entity from Self to dst, for every row that self has. Dropping Components from rows that are not in dst.
    pub fn move_entity_down(&mut self, dst: &mut Self, entity: &Entity) {
        // get entity position
        let position = self.get_entity_position(entity);

        //
        'outer: for current_row in &mut self.rows {
            for dst_row in &mut dst.rows {
                // check if there is a same row type to move to
                if current_row.tid() == dst_row.tid() {
                    current_row.move_push_entity(dst_row, position);

                    continue 'outer;
                }
            }

            // current row type is not existing in dest table
            // drop component
            current_row.swap_remove(position);
        }

        let removed = self.entities.swap_remove(position);
        debug_assert_eq!(removed, *entity);
        dst.entities.push(*entity);
    }

    pub fn try_get_row_ref<C: Component>(&self) -> Result<RowAccessRef<C>, ()> {
        let id = TypeId::of::<C>();
        for row in &self.rows {
            if row.tid() == id {
                return Ok(row.get_access_ref());
            }
        }

        Err(())
    }

    pub fn try_get_row_mut<C: Component>(&self) -> Result<RowAccessMut<C>, ()> {
        let id = TypeId::of::<C>();
        for row in &self.rows {
            if row.tid() == id {
                return Ok(row.get_access_mut());
            }
        }

        Err(())
    }

    #[inline]
    pub const fn id(&self) -> TableId {
        self.id
    }

    #[inline]
    pub fn types(&self) -> impl Iterator<Item = TypeId> {
        self.rows.iter().map(|row| row.tid())
    }

    #[inline]
    pub fn contains_all(&self, types: &[TypeId]) -> bool {
        types.iter().all(|t| {
            for row in &self.rows {
                if row.tid() == *t {
                    return true;
                }
            }
            false
        })
    }

    #[inline]
    fn get_entity_position(&self, entity: &Entity) -> usize {
        self.entities
            .iter()
            .position(|ent| ent == entity)
            .expect("This should have been checked")
    }
}

impl std::fmt::Debug for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::with_capacity(1024);

        use std::fmt::Write;
        _ = writeln!(&mut out, "Table");
        _ = writeln!(&mut out, "    ID: {:?}", self.id);

        for row in &self.rows {
            _ = writeln!(&mut out, "    {} - {:?}: [..]", row.type_name, row.tid());
        }

        _ = writeln!(&mut out, "    ents:    {:?}", self.entities);

        f.write_str(&out)
    }
}

#[derive(Debug)]
pub struct Row {
    type_id: TypeId,
    type_name: &'static str,
    components: AtomicRefCell<Box<RowComponent>>,

    v_clone_empty: fn() -> Self,
    v_swap_remove: fn(row: &mut Row, position: usize),
    v_move_entity: fn(src: &mut Row, dst: &mut Row, position: usize),
}

impl Row {
    pub fn new<C: Component>() -> Self {
        let vec = Vec::<C>::new();
        let boxed: Box<RowComponent> = Box::new(vec);

        Self {
            type_id: TypeId::of::<C>(),
            type_name: type_name::<C>(),
            components: AtomicRefCell::new(boxed),

            v_clone_empty: Self::new::<C>,
            v_swap_remove: Self::v_swap_remove::<C>,
            v_move_entity: Self::v_move_entity::<C>,
        }
    }

    pub fn clone_empty(&self) -> Self {
        (self.v_clone_empty)()
    }

    #[inline]
    pub const fn tid(&self) -> TypeId {
        self.type_id
    }

    pub fn push<C: Component>(&mut self, component: C) {
        self.get_mut().push(component);
    }

    #[allow(clippy::debug_assert_with_mut_call)]
    pub fn update<C: Component>(&mut self, position: usize, component: C) {
        debug_assert!(self.get_mut::<C>().len() > position);

        self.get_mut().insert(position, component);
    }

    pub fn push_or_update<C: Component>(&mut self, position: usize, component: C) {
        let components = self.get_mut::<C>();

        if let Some(current) = components.get_mut(position) {
            *current = component;
        } else {
            debug_assert_eq!(components.len(), position);
            components.push(component);
        }
    }

    #[inline]
    pub fn get_mut<C: Component>(&mut self) -> &mut Vec<C> {
        unwrap!(self.components.get_mut().downcast_mut::<Vec<C>>())
    }

    #[inline]
    pub fn swap_remove(&mut self, position: usize) {
        (self.v_swap_remove)(self, position)
    }

    #[inline]
    pub fn move_push_entity(&mut self, dst: &mut Self, position: usize) {
        (self.v_move_entity)(self, dst, position);
    }

    #[inline]
    pub fn get_access_ref<C: Component>(&self) -> RowAccessRef<C> {
        RowAccessRef {
            guard: self.components.borrow(),
            _p: PhantomData,
        }
    }

    #[inline]
    pub fn get_access_mut<C: Component>(&self) -> RowAccessMut<C> {
        RowAccessMut {
            guard: self.components.borrow_mut(),
            _p: PhantomData,
        }
    }

    fn v_swap_remove<C: Component>(&mut self, position: usize) {
        let vec = unwrap!(self.components.get_mut().downcast_mut::<Vec<C>>());
        vec.swap_remove(position);
    }

    fn v_move_entity<C: Component>(&mut self, dst: &mut Self, position: usize) {
        debug_assert_eq!(self.tid(), dst.tid());

        let removed = self.get_mut::<C>().swap_remove(position);
        dst.get_mut::<C>().push(removed);
    }
}

pub struct ExtendableTable {
    id: TableId,
    rows: Vec<Row>,
    entities: Vec<Entity>,
}

impl ExtendableTable {
    pub fn extend_rows<C: ComponentSet>(&mut self) {
        let new_rows = C::rows();
        for new_row in new_rows {
            // if row does not already exist
            if self.rows.iter().all(|row| row.tid() != new_row.tid()) {
                self.rows.push(new_row);
            }
        }
    }

    pub fn remove_rows<C: ComponentSet>(&mut self) {
        self.rows.retain(|row| !C::contains_type(row.tid()));
    }

    pub fn finish(self) -> Table {
        self.check();

        Table {
            id: self.id,
            rows: self.rows.into_boxed_slice(),
            entities: self.entities,
        }
    }

    #[inline]
    fn check(&self) {
        #[cfg(feature = "runtime-checks")]
        {
            let mut builder = TableIdBuilder::new();
            for t in self.rows.iter().map(Row::tid) {
                builder.add_unqiue(t);
            }
            let id = builder.finish();
            assert_eq!(self.id, id);
        }
    }
}

pub struct RowAccessRef<'a, C: Component> {
    guard: RefGuard<'a, Box<RowComponent>>,
    _p: PhantomData<C>,
}

impl<C: Component> std::ops::Deref for RowAccessRef<'_, C> {
    type Target = [C];

    #[inline]
    fn deref(&self) -> &Self::Target {
        unwrap!(self.guard.downcast_ref::<Vec<C>>())
    }
}

pub struct RowAccessMut<'a, C: Component> {
    guard: MutGuard<'a, Box<RowComponent>>,
    _p: PhantomData<C>,
}

impl<C: Component> std::ops::Deref for RowAccessMut<'_, C> {
    type Target = [C];

    #[inline]
    fn deref(&self) -> &Self::Target {
        unwrap!(self.guard.downcast_ref::<Vec<C>>())
    }
}

impl<C: Component> std::ops::DerefMut for RowAccessMut<'_, C> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unwrap!(self.guard.downcast_mut::<Vec<C>>())
    }
}

#[cfg(test)]
mod tests {

    use std::{any::TypeId, ops::Deref};

    use crate::{
        entity::{Entity, Generation},
        table::RowAccessRef,
    };

    use super::Table;

    #[test]
    fn test_create_table() {
        let table = Table::new::<u32>();
        assert!(table.entities.is_empty());
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].type_id, TypeId::of::<u32>());

        let table = Table::new::<(u32, i32)>();
        assert!(table.entities.is_empty());
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].type_id, TypeId::of::<u32>());
        assert_eq!(table.rows[1].type_id, TypeId::of::<i32>());
    }

    #[cfg(feature = "runtime-checks")]
    #[test]
    #[should_panic]
    fn test_create_table_panic() {
        let table = Table::new::<(u32, u32)>();
        assert!(table.entities.is_empty());
        assert_eq!(table.rows.len(), 2);
    }

    #[test]
    fn test_table_push_and_get() {
        let mut table = Table::new::<(u32, i32)>();
        let ent = Entity::new(0, Generation::new());

        table.push(ent, (100u32, 200i32));

        assert_eq!(&table.entities, &[ent]);
        assert_eq!(table.len(), 1);

        let row = table.try_get_row_ref::<u32>().unwrap();
        let row = RowAccessRef::deref(&row);
        assert_eq!(&row, &[100]);

        let row = table.try_get_row_ref::<i32>().unwrap();
        let row = RowAccessRef::deref(&row);
        assert_eq!(&row, &[200]);
    }

    #[test]
    fn test_push_or_update() {
        let mut table = Table::new::<(u32, i32)>();
        let ent = Entity::new(0, Generation::new());

        table.entities.push(ent);
        table.push_missing_or_update(&ent, (100u32, 200i32));

        assert_eq!(&table.entities, &[ent]);
        assert_eq!(table.len(), 1);

        let row = table.try_get_row_ref::<u32>().unwrap();
        let row = RowAccessRef::deref(&row);
        assert_eq!(&row, &[100]);

        let row = table.try_get_row_ref::<i32>().unwrap();
        let row = RowAccessRef::deref(&row);
        assert_eq!(&row, &[200]);
    }

    #[test]
    fn test_table_move_up() {
        let mut table_single = Table::new::<u32>();
        let mut table_tuple = Table::new::<(u32, i32)>();
        let ent = Entity::new(0, Generation::new());

        table_single.push(ent, 100u32);

        assert_eq!(&table_single.entities, &[ent]);
        assert_eq!(table_single.len(), 1);

        assert_eq!(&table_tuple.entities, &[]);
        assert_eq!(table_tuple.len(), 0);

        table_single.move_entity_up(&mut table_tuple, &ent);
        table_tuple.push_missing_or_update(&ent, 200i32);

        assert_eq!(&table_single.entities, &[]);
        assert_eq!(table_single.len(), 0);

        assert_eq!(&table_tuple.entities, &[ent]);
        assert_eq!(table_tuple.len(), 1);

        let row = table_tuple.try_get_row_ref::<u32>().unwrap();
        let row = RowAccessRef::deref(&row);
        assert_eq!(&row, &[100]);

        let row = table_tuple.try_get_row_ref::<i32>().unwrap();
        let row = RowAccessRef::deref(&row);
        assert_eq!(&row, &[200]);
    }

    #[test]
    fn test_table_move_down() {
        let mut table_single = Table::new::<u32>();
        let mut table_tuple = Table::new::<(u32, i32)>();
        let ent = Entity::new(0, Generation::new());

        table_tuple.push(ent, (100u32, 200i32));

        assert_eq!(&table_tuple.entities, &[ent]);
        assert_eq!(table_tuple.len(), 1);

        assert_eq!(&table_single.entities, &[]);
        assert_eq!(table_single.len(), 0);

        table_tuple.move_entity_down(&mut table_single, &ent);

        assert_eq!(&table_single.entities, &[ent]);
        assert_eq!(table_single.len(), 1);

        assert_eq!(&table_tuple.entities, &[]);
        assert_eq!(table_tuple.len(), 0);

        let row = table_single.try_get_row_ref::<u32>().unwrap();
        let row = RowAccessRef::deref(&row);
        assert_eq!(&row, &[100]);
    }
}
