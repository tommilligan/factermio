#![deny(clippy::all)]
use std::cmp::{max, min};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use rltk::{Console, GameState, Rltk, VirtualKeyCode, RGB};
use specs::prelude::*;
#[macro_use]
extern crate specs_derive;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 50;

fn clamp<T>(value: T, minimum: T, maximum: T) -> T
where
    T: Copy + Ord,
{
    min(maximum, max(minimum, value))
}

#[derive(Debug)]
pub struct Map {
    pub buildings: Vec<Option<Building>>,
}

impl Default for Map {
    fn default() -> Self {
        Self {
            buildings: vec![None; (MAP_WIDTH * MAP_HEIGHT) as usize],
        }
    }
}

impl Map {
    pub fn xy_idx(x: i32, y: i32) -> i32 {
        (y * MAP_WIDTH) + x
    }
}

#[derive(Debug, Component, PartialEq, Eq, Hash, Clone)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn valid(&self) -> bool {
        self.x >= 0 && self.x < MAP_WIDTH && self.y >= 0 && self.y < MAP_HEIGHT
    }
}

#[derive(Debug, Component)]
pub struct Renderable {
    pub glyph: u8,
    pub fg: RGB,
    pub bg: RGB,
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Building {
    Belt,
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Resource {
    Coal,
}

impl Resource {
    pub fn glyph(self) -> char {
        match self {
            Self::Coal => 'c',
        }
    }
}

#[derive(Debug, Component)]
pub struct ResourceBuffer {
    pub resource: Resource,
    pub remaining: i32,
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Component, Debug)]
pub struct Roller {
    pub direction: Direction,
    pub payload: Option<Resource>,
}

impl Roller {
    pub fn glyph(&self) -> char {
        match self.direction {
            Direction::Up => '^',
            Direction::Left => '<',
            Direction::Right => '>',
            Direction::Down => 'v',
        }
    }
}

#[derive(Component, Debug)]
pub struct Player {}

pub struct State {
    pub ecs: World,
    move_roller_resources: MoveRollerResources,
}

impl Default for State {
    fn default() -> Self {
        Self {
            ecs: World::new(),
            move_roller_resources: MoveRollerResources::default(),
        }
    }
}

fn try_move_player(delta_x: i32, delta_y: i32, ecs: &mut World) {
    let mut positions = ecs.write_storage::<Position>();
    let mut players = ecs.write_storage::<Player>();

    for (_player, pos) in (&mut players, &mut positions).join() {
        pos.x = clamp(pos.x + delta_x, 0, MAP_WIDTH - 1);
        pos.y = clamp(pos.y + delta_y, 0, 49);
    }
}

fn player_input(gs: &mut State, ctx: &mut Rltk) {
    // Player movement
    match ctx.key {
        // Nothing happened
        None => {}
        Some(key) => match key {
            VirtualKeyCode::H | VirtualKeyCode::Left => try_move_player(-1, 0, &mut gs.ecs),
            VirtualKeyCode::L | VirtualKeyCode::Right => try_move_player(1, 0, &mut gs.ecs),
            VirtualKeyCode::K | VirtualKeyCode::Up => try_move_player(0, -1, &mut gs.ecs),
            VirtualKeyCode::J | VirtualKeyCode::Down => try_move_player(0, 1, &mut gs.ecs),
            _ => {}
        },
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut Rltk) {
        ctx.cls();

        player_input(self, ctx);
        self.run_systems();

        let positions = self.ecs.read_storage::<Position>();
        let renderables = self.ecs.read_storage::<Renderable>();

        for (pos, render) in (&positions, &renderables).join() {
            ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph);
        }

        //let map = self.ecs.fetch::<Vec<TileType>>();
        //draw_map(&map, ctx);
    }
}

pub struct MoveRollerResources {
    next_update: Instant,
}

impl Default for MoveRollerResources {
    fn default() -> Self {
        Self {
            next_update: Instant::now(),
        }
    }
}

impl<'a> System<'a> for MoveRollerResources {
    type SystemData = (
        ReadStorage<'a, Position>,
        WriteStorage<'a, Roller>,
        WriteStorage<'a, Renderable>,
    );

    fn run(&mut self, (positions, mut rollers, mut renderables): Self::SystemData) {
        let now = Instant::now();
        if self.next_update > now {
            return;
        }
        self.next_update = now + Duration::from_millis(500);

        let mut empty_positions: Vec<Position> = Vec::new();
        let mut target_position_to_source_positions: HashMap<Position, Vec<&Position>> =
            HashMap::new();

        let mut rollers: HashMap<&Position, (&mut Roller, &mut Renderable)> =
            (&positions, &mut rollers, &mut renderables)
                .join()
                .map(|(position, roller, renderable)| {
                    if roller.payload.is_none() {
                        empty_positions.push(position.clone());
                    };
                    let mut target_position = position.clone();
                    match roller.direction {
                        Direction::Up => target_position.y -= 1,
                        Direction::Down => target_position.y += 1,
                        Direction::Left => target_position.x -= 1,
                        Direction::Right => target_position.x += 1,
                    };
                    if target_position.valid() {
                        target_position_to_source_positions
                            .entry(target_position)
                            .or_insert_with(Vec::new)
                            .push(position);
                    };
                    (position, (roller, renderable))
                })
                .collect();

        for empty_position in empty_positions.iter() {
            let mut positions_to_visit = vec![empty_position];

            while let Some(position) = positions_to_visit.pop() {
                if let Some((roller, renderable)) = rollers.remove(position) {
                    if let Some(source_positions) =
                        target_position_to_source_positions.get(position)
                    {
                        positions_to_visit.extend(source_positions);

                        for source_position in source_positions.iter() {
                            if let Some((source_roller, source_renderable)) =
                                rollers.get_mut(source_position)
                            {
                                std::mem::swap(&mut roller.payload, &mut source_roller.payload);
                                renderable.glyph = source_renderable.glyph;
                                source_renderable.glyph = rltk::to_cp437(source_roller.glyph());
                            }
                        }
                    }
                }
            }
        }

        //eprintln!("{:?}", position);
        // let adjustment = if lefty.going_left { -3 } else { 1 };
        // lefty.going_left = !lefty.going_left;
        // pos.x = (pos.x + adjustment).rem_euclid(80);
    }
}

impl State {
    fn run_systems(&mut self) {
        self.move_roller_resources.run_now(&self.ecs);
        self.ecs.maintain();
    }
}
