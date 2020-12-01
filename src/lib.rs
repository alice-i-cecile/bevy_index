use bevy::prelude::*;
use multimap::MultiMap;

use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;

// Look up entities by their value of the component type T
pub struct ComponentIndex<T> {
	// TODO: we can speed this up by changing reverse to be a Hashmap<Entity, Hash<T>>, then feeding those directly back into forward
	// This prevents us from ever having to store the unhashed T, which can be significantly sized
	forward: MultiMap<T, Entity>,
	reverse: HashMap<Entity, T>,
}

impl<T: Hash + Eq> ComponentIndex<T> {
	pub fn get(&self, component: &T) -> Cow<'_, [Entity]> {
		match self.forward.get_vec(component) {
			Some(e) => Cow::from(e),
			None => Cow::from(Vec::new()),
		}
	}

	pub fn new() -> Self {
		ComponentIndex::<T>::default()
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

// TODO: make commands.init_component_index
// which updates index each frame

// TODO: add manual update_index function for multi-stage flow

// IDEA: Can we instead implicitly declare indexes by passing in a ComponentIndex<T> to our systems?
// We don't actually want the full resource structure, since these should never be manually updated
