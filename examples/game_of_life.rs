use bevy::prelude::*;
use bevy_index::{ComponentIndex, ComponentIndexes};

use rand::distributions::{Bernoulli, Distribution};

const MAP_SIZE: isize = 10;
const GAME_INTERVAL: f32 = 0.5;
const FRACTION_ALIVE: f64 = 0.2;

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
struct Position {
	x: isize,
	y: isize,
}

impl Position {
	fn get_neighbors(self) -> Vec<Position> {
		let mut neighbors: Vec<Position> = Vec::new();
		for i in -1..1 {
			for j in -1..1 {
				// A cell is not a neighbor to itself
				if (i != 0) | (j != 0) {
					let candidate_neighbor = Position {
						x: self.x + i,
						y: self.y + j,
					};
					match candidate_neighbor.check_bounds() {
						Some(n) => neighbors.push(n),
						None => (),
					}
				}
			}
		}

		neighbors
	}

	fn check_bounds(self) -> Option<Position> {
		if (0 <= self.x) && (self.x <= MAP_SIZE) && (0 <= self.y) && (self.y <= MAP_SIZE) {
			return Some(self);
		} else {
			return None;
		}
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Life {
	Alive,
	Dead,
}

struct LifeEvent {
	entity: Entity,
	status: Life,
}

struct GameTimer(Timer);

fn main() {
	App::build()
		.add_plugins(DefaultPlugins)
		.add_resource(GameTimer(Timer::from_seconds(GAME_INTERVAL, true)))
		.add_startup_system(init_grid)
		.add_startup_system(init_cells)
		.init_index::<Position>()
		.add_system(game_of_life)
		.add_system_to_stage(stage::POST_UPDATE, process_life_events)
		.run();
}

fn init_grid(commands: &mut Commands) {
	assert!(MAP_SIZE < (usize::MAX as f64).sqrt().floor() as isize);

	// You could do this lazily with itertools::CartesianProduct instead
	let mut positions = Vec::with_capacity((MAP_SIZE * MAP_SIZE) as usize);
	for i in 0..MAP_SIZE {
		for j in 0..MAP_SIZE {
			positions.push(Position { x: i, y: j })
		}
	}

	commands.spawn_batch(positions.into_iter().map(|p| (p, Life::Dead)));
}

fn init_cells(mut query: Query<&mut Life>) {
	let alive_rng = Bernoulli::new(FRACTION_ALIVE).unwrap();

	for mut life in query.iter_mut() {
		if alive_rng.sample(&mut rand::thread_rng()) {
			*life = Life::Alive;
		}
	}
}

fn count_alive(
	neighbors: Vec<Position>,
	position_index: &ComponentIndex<Position>,
	life_query: &Query<&Life>,
) -> u8 {
	neighbors
		.iter()
		.map(|p| {
			position_index
				.get(p)
				.iter()
				.any(|&e| life_query.get(e).ok() == Some(&Life::Alive)) as u8
		})
		.sum()
}

fn game_of_life(
	time: Res<Time>,
	mut timer: ResMut<GameTimer>,
	query: Query<(&Life, &Position, Entity)>,
	position_index: Res<ComponentIndex<Position>>,
	life_query: Query<&Life>,
	mut life_events: ResMut<Events<LifeEvent>>,
) {
	timer.0.tick(time.delta_seconds());
	if timer.0.finished() {
		for (life, position, entity) in query.iter() {
			let n_neighbors = count_alive(position.get_neighbors(), &position_index, &life_query);

			match *life {
				Life::Alive => {
					if (n_neighbors < 2) | (n_neighbors > 3) {
						life_events.send(LifeEvent {
							entity: entity,
							status: Life::Dead,
						})
					}
				}
				Life::Dead => {
					if n_neighbors == 3 {
						life_events.send(LifeEvent {
							entity: entity,
							status: Life::Alive,
						})
					}
				}
			}
		}
	}
}

fn process_life_events(
	mut life_event_reader: Local<EventReader<LifeEvent>>,
	life_events: ResMut<Events<LifeEvent>>,
	mut life_query: Query<&mut Life>,
) {
	for life_event in life_event_reader.iter(&life_events) {
		// Update the entity corresponding with the life_event's entity
		if let Ok(mut life_value) = life_query.get_mut(life_event.entity) {
			*life_value = life_event.status;
		}
	}
}
