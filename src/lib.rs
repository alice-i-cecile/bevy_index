use bevy::prelude::*;
use multimap::MultiMap;

use std::borrow::{Borrow, Cow};
use std::hash::Hash;

pub trait IsIndex {
	type Key;
	type Component;
	fn get_key(c: &Self::Component) -> Self::Key;
}

// Look up entities by their value of the component type T
pub struct ComponentIndex<T: IsIndex + ?Sized> {
	mm: MultiMap<T::Key, Entity>,
}

impl<T: IsIndex + ?Sized> ComponentIndex<T>
where
	T::Key: Hash + Eq,
{
	pub fn get<Q: Hash + Eq + ?Sized>(&self, component: &Q) -> Cow<'_, [Entity]>
	where
		T::Key: Borrow<Q>,
	{
		match self.mm.get_vec(component) {
			Some(e) => Cow::from(e),
			None => Cow::from(Vec::new()),
		}
	}
}

impl<T: IsIndex> Default for ComponentIndex<T>
where
	T::Key: Hash + Eq,
{
	fn default() -> Self {
		Self {
			mm: MultiMap::new(),
		}
	}
}

#[macro_export]
macro_rules! component_indices{
	($name:ident <- $component:ty[$($field:ident : $field_ty:ty),*$(,)?];) => {
		struct $name();
		impl $crate::IsIndex for $name {
			type Component = $component;
			#[allow(unused_parens)]
			type Key = ($($field_ty),*);
			fn get_key(c: &Self::Component) -> Self::Key {
			    ($(c.$field),*).clone()
			}
		}
	}
}

// TODO: Write derive macro for simple case

#[cfg(test)]
mod test {
	use super::*;

	#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
	struct Position {
		x: isize,
		y: isize,
	}
	impl IsIndex for Position {
		type Component = Position;
		type Key = Position;
		fn get_key(pos: &Position) -> Self::Key {
			pos.clone()
		}
	}

	struct InWorld {
		pos: Position,
		/* whatever */
	}

	component_indices! {
		WorldPosition <- InWorld[pos: Position];
	}
}

// Alternate idea:
//
// component_indices!{
//	 WorldPosition <- World[pos: Position]
// };

// no matter what you will probably want a trait for valid index types
// with a corresponding underlying type because otherwise you are limited
// to only one index on each field type which is kinda naff but you also
// don't want to force you to wrap/unwrap the underlying index type

// app.init_component_index<PositionIndex<InWorld>>()

// Idea: create an IsRefIndex trait, that implements an extract_key_ref

// TODO: make commmands.init_component_index
// which updates index each frame

// TODO: add manual update_index function for multi-stage flow
