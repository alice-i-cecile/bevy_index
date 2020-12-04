use bevy::prelude::*;
use bevy_index::{ComponentIndex, ComponentIndexable};

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
enum Shape{
	Square,
	Star,
	Circle,
	Moon
}
#[derive(Debug)]
struct Score {
	val: isize
}

fn main() {
	App::build()
	.init_index::<Shape>()
	.add_startup_system(create_tokens)
	.add_system(show_star_score)
	.run()
}

fn create_tokens(commands: &mut Commands){
	commands
		.spawn((Shape::Square, Score {val: 0}))
		.spawn((Shape::Star, Score {val: 1}))
		.spawn((Shape::Circle, Score {val: 2}))
		.spawn((Shape::Moon, Score {val: 3}))
		.spawn((Shape::Square, Score {val: 4}))
		.spawn((Shape::Star, Score {val: 5}))
		.spawn((Shape::Circle, Score {val: 6}))
		.spawn((Shape::Moon, Score {val: 7}));
}

fn show_star_score(query: Query<&Score>, shape_index: Res<ComponentIndex<Shape>>){
	let stars = shape_index.get(&Shape::Star);

	for star in stars.iter(){
		let star_score = query.get_component::<Score>(*star);

		// For all components within the query, instead use
		// let star_components = query.get(*star)

		match star_score {
			Ok(s) => println!("Star {:?} has a score of {:?}", star, s.val),
			Err(_) => println!("Error when attempting to find entity {:?}", star) 
		};
	}
}