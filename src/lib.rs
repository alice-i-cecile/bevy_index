use bevy::prelude::*;
use multimap::MultiMap;

use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;

// Look up entities by their value of the component type T
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

	fn remove(&mut self, entity: &Entity){
		let old_component = &self.reverse.get(&entity);
		if old_component.is_some() {
			self
				.forward
				.retain(|k, v| (k == old_component.unwrap()) && (v != entity));
			self.reverse.remove(entity);
		}
	} 
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
	// TODO: add manual update_index function for multi-stage flow

	// TODO: add clean function to remove unused keys and fix memory locality

}


// IDEA: Can we instead implicitly declare indexes by passing in a ComponentIndex<T> to our systems?
// We don't actually want the full resource structure, since these should never be manually updated

mod test {

	// Basic values

	// New values

	// Removed values

	// Changed values

	// Enums

	// Structs

	// TupleStructs
}
