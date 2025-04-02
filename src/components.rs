use std::any::{Any, TypeId};

use crate::{
    entity::{Entity, EntitySpawner, Generation},
    macros::unwrap,
    table::{ExtendableTable, Table, TableId, TableIdent},
};

pub trait Component: Any + Send + Sync {}

/// A trait representing a type erased component.
pub type UntypedComponentSet = dyn Any + Send + Sync;

pub trait ComponentSet: TableIdent + Send + Sync + 'static {
    /// Returns all the types this ComponentSet contains.
    fn types() -> Vec<TypeId>;

    fn contains_type(type_id: TypeId) -> bool;

    /// Add Self to a table for a given Entity.
    fn push_to_table(self, table: &mut Table, entity: Entity)
    where
        Self: Sized;

    /// Overrides currently existing components with Self for a given Entity.
    fn update_rows(self, table: &mut Table, position: usize);

    fn push_or_update(self, table: &mut Table, position: usize);
}

#[derive(Default)]
pub struct EntityComponents {
    pub(crate) tables: Vec<Table>,
    pub(crate) entities: Vec<(Generation, TableId)>,
    spawner: EntitySpawner,
}

#[cfg(feature = "debug-utils")]
impl std::fmt::Debug for EntityComponents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntityComponents")
            .field("tables", &self.tables)
            .field("entities", &self.entities)
            .field("spawner", &self.spawner)
            .finish()
    }
}

impl EntityComponents {
    pub fn new() -> Self {
        Self {
            tables: Vec::new(),
            entities: Vec::new(),
            spawner: EntitySpawner::new(),
        }
    }

    #[inline]
    pub fn spawner(&self) -> EntitySpawner {
        self.spawner.clone()
    }

    pub fn spawn_entity(&mut self) -> Entity {
        // reserve entity
        let entity = self.spawner.reserve();

        // activate entity
        self.activate_entity(entity);

        entity
    }

    /// Activates a previously reserved `Entity` via commands.
    pub fn activate_entity(&mut self, entity: Entity) {
        // don't active invalid entity
        debug_assert!(!entity.generation().is_invalid());

        // set entity generation
        if self.entities.len() <= entity.id() {
            self.entities.resize(
                (entity.id() + 1).next_power_of_two(),
                (Generation::invalid(), TableId::invalid()),
            );
        }

        // only activate invalid entity
        debug_assert!(self.entities[entity.id()].0.is_invalid());
        debug_assert!(self.entities[entity.id()].1.is_invalid());

        // update/set generation
        self.entities[entity.id()].0 = entity.generation();
    }

    pub fn delete_entity(&mut self, entity: Entity) {
        // look up in what table the entity is
        let (generation, table_id) = unwrap!(self.entities.get_mut(entity.id()));

        // entity is not valid
        let ent_gen = entity.generation();
        if ent_gen.is_invalid() || ent_gen != *generation {
            return;
        }

        // find table
        let (pos, table) = unwrap!(
            self.tables
                .iter_mut()
                .enumerate()
                .find(|(_, table)| table.id() == *table_id)
        );

        // delete entity from table
        table.delete_entity(entity);

        // unset table-link
        self.entities[pos].1 = TableId::invalid();

        self.spawner.free(entity);

        // if table is empty, remove it?
        if table.is_empty() {
            self.tables.swap_remove(pos);
        }
    }

    pub fn add_components<C: ComponentSet>(&mut self, entity: &Entity, components: C) {
        // try to find entity
        let (generation, in_table) = match self.entities.get_mut(entity.id()) {
            Some((generation, in_table)) => (generation, in_table),
            None => return,
        };

        // check entity validity
        if entity.generation() != *generation || generation.is_invalid() {
            return;
        }

        // get TableId for components
        let component_table_id = C::table_id();

        // entity has no components
        if in_table.is_invalid() {
            // get TableId for components

            *in_table = component_table_id;

            match self
                .tables
                .iter_mut()
                .position(|table| table.id() == component_table_id)
            {
                Some(table_i) => {
                    // insert directly in correct table
                    let target_table = unwrap!(self.tables.get_mut(table_i));
                    target_table.push(*entity, components);
                    return;
                }
                None => {
                    // create new table, add to table
                    let mut new_table = Table::new::<C>();
                    new_table.push(*entity, components);

                    // insert new table in table list
                    self.tables.push(new_table);
                    return;
                }
            }
        }

        // get current table
        let current_table_i = unwrap!(
            self.tables
                .iter_mut()
                .position(|table| table.id() == *in_table)
        );

        let current_table = unwrap!(self.tables.get_mut(current_table_i));

        // same components, just update table
        if *in_table == component_table_id {
            current_table.update::<C>(entity, components);
            return;
        }

        // ComponentSet is subset of current table (no move, just update/override)
        let types = C::types();
        if current_table.contains_all(&types) {
            current_table.update_partial::<C>(entity, components);
            return;
        }

        // #! entity has components, adds additional (potential overlapping) components

        // compute types and TableId
        let mut set = types;
        for t in current_table.types() {
            // insert uniques
            if !set.contains(&t) {
                set.push(t);
            }
        }
        let target_table_id = TableId::from_uniques(set.iter());

        // find fitting table
        let target_table_index = self
            .tables
            .iter_mut()
            .position(|table| table.id() == target_table_id);

        // find table to push ComponentSet in
        let target_table_i = target_table_index.unwrap_or_else(|| {
            let current_table = unwrap!(self.tables.get(current_table_i));

            // get a fresh/empty clone of the current table
            // use already computed types and id here
            let mut extend = current_table.get_extendable_precomputed(target_table_id);

            // extend the table based on the current ComponentSet
            extend.extend_rows::<C>();
            let new_table = extend.finish();

            // insert new table in table list
            let i = self.tables.len();
            self.tables.push(new_table);
            i
        });

        // get disjoint
        let [current_table, target_table] = unwrap!(
            self.tables
                .get_disjoint_mut([current_table_i, target_table_i])
        );

        // move entity and components from current table to target table
        current_table.move_entity_up(target_table, entity);

        // push missing component and/or override already existing
        target_table.push_missing_or_update(entity, components);

        *in_table = target_table_id;
    }

    pub fn add_component_untyped(
        &mut self,
        entity: &Entity,
        components: Box<UntypedComponentSet>,
        modifier: ComponentAddModifier,
    ) {
        // try to find entity
        let (generation, in_table) = match self.entities.get_mut(entity.id()) {
            Some((generation, in_table)) => (generation, in_table),
            None => return,
        };

        // check entity validity
        if entity.generation() != *generation || generation.is_invalid() {
            return;
        }

        // get TableId for components
        let component_table_id = (modifier.table_id)();

        // entity has no components
        if in_table.is_invalid() {
            // get TableId for components

            *in_table = component_table_id;

            match self
                .tables
                .iter_mut()
                .position(|table| table.id() == component_table_id)
            {
                Some(table_i) => {
                    // insert directly in correct table
                    let target_table = unwrap!(self.tables.get_mut(table_i));

                    (modifier.push)(target_table, *entity, components);

                    return;
                }
                None => {
                    // create new table, add to table
                    let mut new_table = (modifier.table_new)();
                    (modifier.push)(&mut new_table, *entity, components);

                    // insert new table in table list
                    self.tables.push(new_table);
                    return;
                }
            }
        }

        // get current table
        let current_table_i = unwrap!(
            self.tables
                .iter_mut()
                .position(|table| table.id() == *in_table)
        );

        let current_table = unwrap!(self.tables.get_mut(current_table_i));

        // same components, just update table
        if *in_table == component_table_id {
            (modifier.update)(current_table, entity, components);
            return;
        }

        // ComponentSet is subset of current table (no move, just update/override)
        let types = (modifier.types)();
        if current_table.contains_all(&types) {
            (modifier.update_partial)(current_table, entity, components);
            return;
        }

        // #! entity has components, adds additional (potential overlapping) components

        // compute types and TableId
        let mut set = types;
        for t in current_table.types() {
            // insert uniques
            if !set.contains(&t) {
                set.push(t);
            }
        }
        let target_table_id = TableId::from_uniques(set.iter());

        // find fitting table
        let target_table_index = self
            .tables
            .iter_mut()
            .position(|table| table.id() == target_table_id);

        // find table to push ComponentSet in
        let target_table_i = target_table_index.unwrap_or_else(|| {
            let current_table = unwrap!(self.tables.get(current_table_i));

            // get a fresh/empty clone of the current table
            // use already computed types and id here
            let mut extend = current_table.get_extendable_precomputed(target_table_id);

            // extend the table based on the current ComponentSet
            (modifier.extend_rows)(&mut extend);
            let new_table = extend.finish();

            // insert new table in table list
            let i = self.tables.len();
            self.tables.push(new_table);
            i
        });

        // get disjoint
        let [current_table, target_table] = unwrap!(
            self.tables
                .get_disjoint_mut([current_table_i, target_table_i])
        );

        // move entity and components from current table to target table
        current_table.move_entity_up(target_table, entity);

        // push missing component and/or override already existing
        (modifier.push_missing_or_update)(target_table, entity, components);

        *in_table = target_table_id;
    }

    pub fn remove_component<C: ComponentSet>(&mut self, entity: &Entity) {
        // try to find entity
        let (generation, in_table) = match self.entities.get_mut(entity.id()) {
            Some((generation, in_table)) => (generation, in_table),
            None => return,
        };

        // check entity validity
        if entity.generation() != *generation || generation.is_invalid() {
            return;
        }

        // check if table is valid
        if in_table.is_invalid() {
            return;
        }

        let current_table_i = unwrap!(self.tables.iter().position(|table| table.id() == *in_table));

        let current_table = &self.tables[current_table_i];

        let new_types = current_table
            .types()
            .filter(|t| !C::contains_type(*t))
            .collect::<Vec<_>>();

        // if all components are removed from entity
        if new_types.is_empty() {
            let current_table = &mut self.tables[current_table_i];
            current_table.delete_entity(*entity);
            *in_table = TableId::invalid();
            return;
        }

        let target_table_id = TableId::from_uniques(new_types.iter());

        let target_table_i = self
            .tables
            .iter()
            .position(|table| table.id() == target_table_id)
            .unwrap_or_else(|| {
                let current_table = unwrap!(self.tables.get(current_table_i));

                // get a fresh/empty clone of the current table
                // use already computed types and id here
                let mut extend = current_table.get_extendable_precomputed(target_table_id);

                // remove all rows belonging to C
                extend.remove_rows::<C>();

                let new_table = extend.finish();

                // insert new table in table list
                let i = self.tables.len();
                self.tables.push(new_table);
                i
            });

        // get disjoint
        let [current_table, target_table] = unwrap!(
            self.tables
                .get_disjoint_mut([current_table_i, target_table_i])
        );

        current_table.move_entity_down(target_table, entity);

        *in_table = target_table_id;
    }

    pub fn remove_components_untyped(
        &mut self,
        entity: &Entity,
        modifier: ComponentRemoveModifier,
    ) {
        // try to find entity
        let (generation, in_table) = match self.entities.get_mut(entity.id()) {
            Some((generation, in_table)) => (generation, in_table),
            None => return,
        };

        // check entity validity
        if entity.generation() != *generation || generation.is_invalid() {
            return;
        }

        // check if table is valid
        if in_table.is_invalid() {
            return;
        }

        let current_table_i = unwrap!(self.tables.iter().position(|table| table.id() == *in_table));

        let current_table = &self.tables[current_table_i];

        let new_types = current_table
            .types()
            .filter(|t| !(modifier.contains_type)(*t))
            .collect::<Vec<_>>();

        // if all components are removed from entity
        if new_types.is_empty() {
            let current_table = &mut self.tables[current_table_i];
            current_table.delete_entity(*entity);
            *in_table = TableId::invalid();
            return;
        }

        let target_table_id = TableId::from_uniques(new_types.iter());

        let target_table_i = self
            .tables
            .iter()
            .position(|table| table.id() == target_table_id)
            .unwrap_or_else(|| {
                let current_table = unwrap!(self.tables.get(current_table_i));

                // get a fresh/empty clone of the current table
                // use already computed types and id here
                let mut extend = current_table.get_extendable_precomputed(target_table_id);

                // remove all rows belonging to C
                (modifier.remove_rows)(&mut extend);

                let new_table = extend.finish();

                // insert new table in table list
                let i = self.tables.len();
                self.tables.push(new_table);
                i
            });

        // get disjoint
        let [current_table, target_table] = unwrap!(
            self.tables
                .get_disjoint_mut([current_table_i, target_table_i])
        );

        current_table.move_entity_down(target_table, entity);

        *in_table = target_table_id;
    }
}

pub struct ComponentAddModifier {
    table_id: fn() -> TableId,
    types: fn() -> Vec<TypeId>,
    table_new: fn() -> Table,
    push: fn(&mut Table, Entity, Box<UntypedComponentSet>),
    update: fn(&mut Table, &Entity, Box<UntypedComponentSet>),
    update_partial: fn(&mut Table, &Entity, Box<UntypedComponentSet>),
    extend_rows: fn(&mut ExtendableTable),
    push_missing_or_update: fn(&mut Table, &Entity, Box<UntypedComponentSet>),
}

impl ComponentAddModifier {
    #[inline]
    pub const fn new<C: ComponentSet>() -> Self {
        Self {
            table_id: C::table_id,
            types: C::types,
            table_new: Table::new::<C>,
            push: Self::ptf_push::<C>,
            update: Self::ptf_update::<C>,
            update_partial: Self::ptf_update_partial::<C>,
            extend_rows: Self::ptf_extend_rows::<C>,
            push_missing_or_update: Self::ptf_push_missing_or_update::<C>,
        }
    }

    fn ptf_push<C: ComponentSet>(
        table: &mut Table,
        entity: Entity,
        components: Box<UntypedComponentSet>,
    ) {
        let components = *unwrap!(components.downcast::<C>());

        table.push(entity, components);
    }

    fn ptf_update<C: ComponentSet>(
        table: &mut Table,
        entity: &Entity,
        components: Box<UntypedComponentSet>,
    ) {
        let components = *unwrap!(components.downcast::<C>());

        table.update(entity, components);
    }

    fn ptf_update_partial<C: ComponentSet>(
        table: &mut Table,
        entity: &Entity,
        components: Box<UntypedComponentSet>,
    ) {
        let components = *unwrap!(components.downcast::<C>());

        table.update_partial(entity, components);
    }

    fn ptf_extend_rows<C: ComponentSet>(table: &mut ExtendableTable) {
        table.extend_rows::<C>();
    }

    fn ptf_push_missing_or_update<C: ComponentSet>(
        table: &mut Table,
        entity: &Entity,
        components: Box<UntypedComponentSet>,
    ) {
        let components = *unwrap!(components.downcast::<C>());

        table.push_missing_or_update(entity, components);
    }
}

pub struct ComponentRemoveModifier {
    contains_type: fn(TypeId) -> bool,
    remove_rows: fn(&mut ExtendableTable),
}

impl ComponentRemoveModifier {
    pub const fn new<C: ComponentSet>() -> Self {
        Self {
            contains_type: Self::ptf_contanins_type::<C>,
            remove_rows: Self::ptf_remove_rows::<C>,
        }
    }

    fn ptf_contanins_type<C: ComponentSet>(type_id: TypeId) -> bool {
        C::contains_type(type_id)
    }

    fn ptf_remove_rows<C: ComponentSet>(table: &mut ExtendableTable) {
        table.remove_rows::<C>();
    }
}
