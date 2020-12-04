use bevy::prelude::*;
use multimap::MultiMap;

use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;

// IDEA: Can we instead implicitly declare indexes by passing in a ComponentIndex<T> to our systems?
// We don't actually want the full resource structure, since these should never be manually updated
pub struct ComponentIndex<T> {
    // TODO: we can speed this up by changing reverse to be a Hashmap<Entity, Hash<T>>, then feeding those directly back into forward
    // This prevents us from ever having to store the unhashed T, which can be significantly sized (requires unstable functionality)

    // TODO: How can we improve memory locality on this data structure
    forward: MultiMap<T, Entity>,
    reverse: HashMap<Entity, T>,
}

impl<T: Hash + Eq> ComponentIndex<T> {
    pub fn get(&self, component_val: &T) -> Cow<'_, [Entity]> {
        match self.forward.get_vec(component_val) {
            Some(e) => Cow::from(e),
            None => Cow::from(Vec::new()),
        }
    }

    pub fn new() -> Self {
        ComponentIndex::<T>::default()
    }

    fn remove(&mut self, entity: &Entity) {
        let old_component = &self.reverse.get(&entity);
        if old_component.is_some() {
            self.forward
                .retain(|k, v| (k == old_component.unwrap()) && (v != entity));
            self.reverse.remove(entity);
        }
	}
	
	// TODO: add manual_update function for multi-stage flow

    // TODO: add clean function to remove unused keys and fix memory locality

}

impl<T: Hash + Eq> Default for ComponentIndex<T> {
    fn default() -> Self {
        ComponentIndex::<T> {
            forward: MultiMap::new(),
            reverse: HashMap::new(),
        }
    }
}

pub trait IndexKey: Component + Eq + Hash + Clone {}
impl<T: Component + Eq + Hash + Clone> IndexKey for T {}

pub trait ComponentIndexes {
    fn init_index<T: IndexKey>(&mut self) -> &mut Self;

    fn update_component_index<T: IndexKey>(
        index: ResMut<ComponentIndex<T>>,
        query: Query<(&T, Entity)>,
        changed_query: Query<(&T, Entity), Changed<T>>,
    );
}

impl ComponentIndexes for AppBuilder {
    fn init_index<T: IndexKey>(&mut self) -> &mut Self {
        self.init_resource::<ComponentIndex<T>>();
        self.add_startup_system_to_stage("post_startup", Self::update_component_index::<T>);
        self.add_system_to_stage(stage::POST_UPDATE, Self::update_component_index::<T>);

        self
    }

    fn update_component_index<T: IndexKey>(
        mut index: ResMut<ComponentIndex<T>>,
        query: Query<(&T, Entity)>,
        changed_query: Query<(&T, Entity), Changed<T>>,
    ) {
        // First, clean up any entities who had this component removed
        for entity in query.removed::<T>().iter() {
            index.remove(entity);
        }

        for (component, entity) in changed_query.iter() {
            index.remove(&entity);

            // Add in new values for the changed records to the forward and reverse entries
            index.forward.insert(component.clone(), entity);
            index.reverse.insert(entity, component.clone());
        }
    }
}

#[allow(dead_code)]
mod test {

    use super::*;
    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    struct MyStruct {
        val: i8,
    }

    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    struct MyTupleStruct(i8);
    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    struct MyCompoundStruct {
        val: i8,
        name: String,
    }
    #[allow(dead_code)]
    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    enum MyEnum {
        Red,
        Blue,
    }

    const GOOD_NUMBER: i8 = 42;
    const BAD_NUMBER: i8 = 0;

    #[derive(Debug, PartialEq, Eq)]
    enum Goodness {
        Good,
        Bad,
        Confused,
    }

    fn spawn_bad_entity(commands: &mut Commands) {
        commands.spawn((MyStruct { val: BAD_NUMBER }, Goodness::Bad));
    }

    fn spawn_good_entity(commands: &mut Commands) {
        commands.spawn((MyStruct { val: GOOD_NUMBER }, Goodness::Good));
    }

    fn spawn_deficient_entity(commands: &mut Commands) {
        commands.spawn((Goodness::Good,));
    }

    fn augment_entities(
        commands: &mut Commands,
        query: Query<Entity, (With<Goodness>, Without<MyStruct>)>,
    ) {
        for e in query.iter() {
            commands.insert(e, (MyStruct { val: GOOD_NUMBER },));
        }
    }

    fn reform_entities(
        mut query: Query<(&mut Goodness, &mut MyStruct)>,
        index: Res<ComponentIndex<MyStruct>>,
    ) {
        let entities = index.get(&MyStruct { val: BAD_NUMBER });

        for e in entities.iter() {
            let (mut goodness, mut val) = query.get_mut(*e).unwrap();
            *goodness = Goodness::Good;
            *val = MyStruct { val: GOOD_NUMBER };
        }
    }

    fn purge_badness(commands: &mut Commands, index: Res<ComponentIndex<MyStruct>>) {
        let entities = index.get(&MyStruct { val: BAD_NUMBER });

        for e in entities.iter() {
            commands.despawn(*e);
        }
    }

    fn ensure_goodness(query: Query<&Goodness>, index: Res<ComponentIndex<MyStruct>>) {
        let entities = index.get(&MyStruct { val: GOOD_NUMBER });

        // Each test must have at least one matching example when checked
        assert!(entities.len() >= 1);

        // Each entity with MyStruct.val = GOOD_NUMBER is Good
        for e in entities.iter() {
            assert_eq!(
                query
                    .get_component::<Goodness>(*e)
                    .unwrap_or(&Goodness::Confused),
                &Goodness::Good
            );
        }
    }

    fn ensure_absence_of_bad(query: Query<&Goodness>, index: Res<ComponentIndex<MyStruct>>) {
        let entities = index.get(&MyStruct { val: BAD_NUMBER });

        assert!(entities.len() == 0);

        for goodness in query.iter() {
            assert!(*goodness != Goodness::Bad);
        }
    }

    #[test]
    fn struct_test() {
        App::build().init_index::<MyStruct>().run()
    }

    #[test]
    fn tuple_struct_test() {
        App::build().init_index::<MyTupleStruct>().run()
    }
    #[test]
    fn compound_struct_test() {
        App::build().init_index::<MyCompoundStruct>().run()
    }
    #[test]
    fn enum_test() {
        App::build().init_index::<MyEnum>().run()
    }

    #[test]
    fn startup_spawn_test() {
        App::build()
            .init_index::<MyStruct>()
            .add_startup_system(spawn_good_entity)
            .add_startup_system(spawn_bad_entity)
            .add_system_to_stage(stage::FIRST, ensure_goodness)
            .run()
    }

    #[test]
    fn update_spawn_test() {
        App::build()
            .init_index::<MyStruct>()
            .add_system(spawn_good_entity)
            .add_system(spawn_bad_entity)
            .add_system_to_stage(stage::LAST, ensure_goodness)
            .run()
    }

    #[test]
    fn duplicate_spawn_test() {
        App::build()
            .init_index::<MyStruct>()
            .add_system(spawn_good_entity)
            .add_system(spawn_good_entity)
            .add_system(spawn_bad_entity)
            .add_system_to_stage(stage::LAST, ensure_goodness)
            .run()
    }

    #[test]
    fn component_added_test() {
        App::build()
            .init_index::<MyStruct>()
            .add_startup_system(spawn_deficient_entity)
            .add_startup_system(spawn_bad_entity)
            .add_system(augment_entities)
            .add_system_to_stage(stage::LAST, ensure_goodness)
            .run()
    }
    #[test]
    fn component_modified_test() {
        App::build()
            .init_index::<MyStruct>()
            .add_startup_system(spawn_bad_entity)
            .add_startup_system(spawn_bad_entity)
            .add_system(reform_entities)
            .add_system_to_stage(stage::LAST, ensure_goodness)
            .run()
    }

    #[test]
    fn entity_removal_test() {
        App::build()
            .init_index::<MyStruct>()
            .add_startup_system(spawn_bad_entity)
            .add_system(purge_badness)
            .add_system_to_stage(stage::LAST, ensure_absence_of_bad)
            .run()
    }

    #[test]
    fn duplicate_removal_test() {
        App::build()
            .init_index::<MyStruct>()
            .add_startup_system(spawn_bad_entity)
            .add_startup_system(spawn_bad_entity)
            .add_system(purge_badness)
            .add_system_to_stage(stage::LAST, ensure_absence_of_bad)
            .run()
    }

    #[test]
    fn same_stage_addition_test() {
        App::build()
            .init_index::<MyStruct>()
            .add_system(spawn_deficient_entity)
            .add_system(augment_entities)
            .add_system_to_stage(stage::LAST, ensure_goodness)
            .run()
    }

    #[test]
    fn same_stage_modification_test() {
        App::build()
            .init_index::<MyStruct>()
            .add_system(spawn_bad_entity)
            .add_system(reform_entities)
            .add_system_to_stage(stage::LAST, ensure_goodness)
            .run()
    }

    #[test]
    fn same_stage_removal_test() {
        App::build()
            .init_index::<MyStruct>()
            .add_system(spawn_bad_entity)
            .add_system(purge_badness)
            .add_system_to_stage(stage::LAST, ensure_absence_of_bad)
            .run()
	}
	
	#[test]
	fn earlier_stage_addition_test() {
        App::build()
            .init_index::<MyStruct>()
            .add_system_to_stage(stage::PRE_UPDATE, spawn_deficient_entity)
            .add_system(augment_entities)
            .add_system_to_stage(stage::LAST, ensure_goodness)
            .run()
	}
	
	#[test]
	#[should_panic]
    fn reverse_addition_test() {
        App::build()
            .init_index::<MyStruct>()
			.add_system(augment_entities)
			.add_system(spawn_deficient_entity)
            .add_system_to_stage(stage::LAST, ensure_goodness)
            .run()
    }
}
