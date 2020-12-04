use bevy::prelude::*;
use bevy_index::{ComponentIndex, ComponentIndexable};

#[derive(Clone, Hash, PartialEq, Eq)]
struct Name(&'static str);
#[derive(Debug)]
struct Score(isize);

fn main() {
	App::build()
	.init_index::<Name>()
	.add_startup_system(create_npcs)
	.add_system(get_cart_score)
	.run()
}

fn create_npcs(commands: &mut Commands){
	commands
		.spawn((Name("Alice"), Score (0)))
		.spawn((Name("Bevy"),  Score (1)))
		.spawn((Name("Cart"),  Score (2)));
}

fn get_cart_score(query: Query<&Score>, name_index: Res<ComponentIndex<Name>>){
	let carts = name_index.get(&Name("Cart"));

	for cart in carts.iter(){
		let cart_score = query.get_component::<Score>(*cart);

		// For all components within the query, instead use
		// let alice_components = query.get(*alice)

		match cart_score {
			Ok(s) => println!("The entity {:?} named Cart has a score of {:?}.", cart, s.0),
			Err(_) => println!("Error when attempting to find entity {:?}.", cart) 
		};
	}
}