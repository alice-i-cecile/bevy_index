use bevy::app::startup_stage;
use bevy::prelude::*;
use bevy_index::{ComponentIndex, ComponentIndexes};

use rand::distributions::{Bernoulli, Distribution};

const MAP_SIZE: isize = 10;
const GAME_INTERVAL: f32 = 0.5;
const FRACTION_ALIVE: f64 = 0.2;

const GRAPHICS_SCALE: f32 = 10.0;
const COL_ALIVE: Color = Color::rgb_linear(0.0, 0.0, 0.0);
const COL_DEAD: Color = Color::rgb_linear(1.0, 1.0, 1.0);

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

#[derive(Debug, Clone)]
struct LifeEvent {
    entity: Entity,
    status: Life,
}

struct GameTimer(Timer);

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_resource(GameTimer(Timer::from_seconds(GAME_INTERVAL, true)))
        .init_index::<Position>()
        .add_event::<LifeEvent>()
        .add_startup_system(init_camera.system())
        .add_startup_system(init_grid.system())
        .add_startup_system_to_stage(startup_stage::POST_STARTUP, init_cells.system())
        //.add_system(report_alive.system())
        .add_system(game_of_life.system())
        .add_system_to_stage(stage::POST_UPDATE, process_life_events.system())
        .add_system_to_stage(stage::LAST, update_cell_color.system())
        .run();
}

fn init_grid(commands: &mut Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    assert!(MAP_SIZE < (usize::MAX as f64).sqrt().floor() as isize);

    // spawn_batch doesn't work because ColorMaterial isn't thread safe
    for x in -MAP_SIZE..MAP_SIZE {
        for y in -MAP_SIZE..MAP_SIZE {
            commands
                .spawn(SpriteBundle {
                    material: materials.add(COL_DEAD.into()),
                    transform: Transform::from_translation(Vec3::new(
                        x as f32 * GRAPHICS_SCALE,
                        y as f32 * GRAPHICS_SCALE,
                        0.0,
                    )),
                    sprite: Sprite::new(Vec2::new(0.9 * GRAPHICS_SCALE, 0.9 * GRAPHICS_SCALE)),
                    ..Default::default()
                })
                .with(Position { x, y })
                .with(Life::Dead);
        }
    }
}

fn init_cells(mut query: Query<&mut Life>) {
    let alive_rng = Bernoulli::new(FRACTION_ALIVE).unwrap();

    for mut life in query.iter_mut() {
        if alive_rng.sample(&mut rand::thread_rng()) {
            *life = Life::Alive;
        }
    }
}

fn init_camera(commands: &mut Commands) {
    commands.spawn(Camera2dBundle::default());
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

// FIXME: kills all neighboring cells
// Pretty sure it's because the index isn't updated in time
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
            // FIXME:
            let n_neighbors = count_alive(position.get_neighbors(), &position_index, &life_query);
            dbg!(n_neighbors);

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
        dbg!(life_event.status);

        // Update the entity corresponding with the life_event's entity
        if let Ok(mut life_value) = life_query.get_mut(life_event.entity) {
            *life_value = life_event.status;
        }
    }
}

// FIXME: isn't working
fn update_cell_color(mut query: Query<(&Life, &mut ColorMaterial), Changed<Life>>) {
    for (life, mut color) in query.iter_mut() {
        *color = match life {
            Life::Alive => COL_ALIVE.into(),
            Life::Dead => COL_DEAD.into(),
        }
    }
}

fn report_alive(query: Query<&Life>) {
    let mut n = 0;
    for life in query.iter() {
        if *life == Life::Alive {
            n += 1;
        }
    }
    println!("{}", n);
}
