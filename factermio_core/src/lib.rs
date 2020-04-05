#![deny(clippy::all)]
use std::cmp::{max, min};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use lazy_static::lazy_static;
use rltk::{Console, GameState, Rltk, VirtualKeyCode, RGB};
use specs::prelude::*;
#[macro_use]
extern crate specs_derive;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 50;

lazy_static! {
    static ref COAL_FOREGROUND: Foreground = Foreground {
        glyph: rltk::to_cp437('c'),
        color: RGB::named(rltk::BLACK),
    };
    static ref BELT_UP_FOREGROUND: Foreground = Foreground {
        glyph: rltk::to_cp437('^'),
        color: RGB::named(rltk::YELLOW),
    };
    static ref BELT_DOWN_FOREGROUND: Foreground = Foreground {
        glyph: rltk::to_cp437('v'),
        color: RGB::named(rltk::YELLOW),
    };
    static ref BELT_LEFT_FOREGROUND: Foreground = Foreground {
        glyph: rltk::to_cp437('<'),
        color: RGB::named(rltk::YELLOW),
    };
    static ref BELT_RIGHT_FOREGROUND: Foreground = Foreground {
        glyph: rltk::to_cp437('>'),
        color: RGB::named(rltk::YELLOW),
    };
}

fn clamp<T>(value: T, minimum: T, maximum: T) -> T
where
    T: Copy + Ord,
{
    min(maximum, max(minimum, value))
}

#[derive(Debug)]
pub struct Map {
    pub buildings: Vec<Option<BuildingKind>>,
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

#[derive(Debug, Clone, Copy)]
pub struct Foreground {
    pub glyph: u8,
    pub color: RGB,
}

impl From<Resource> for Foreground {
    fn from(resource: Resource) -> Self {
        match resource {
            Resource::Coal => *COAL_FOREGROUND,
        }
    }
}

impl From<&ResourceMover> for Foreground {
    fn from(belt: &ResourceMover) -> Self {
        match belt.payload {
            Some(payload) => payload.into(),
            None => match belt.direction {
                Direction::Up => *BELT_UP_FOREGROUND,
                Direction::Down => *BELT_DOWN_FOREGROUND,
                Direction::Left => *BELT_LEFT_FOREGROUND,
                Direction::Right => *BELT_RIGHT_FOREGROUND,
            },
        }
    }
}

#[derive(Debug, Component)]
pub struct Renderable {
    pub glyph: u8,
    pub fg: RGB,
    pub bg: RGB,
}

impl Renderable {
    pub fn merge_foreground(&mut self, foreground: Foreground) {
        self.fg = foreground.color;
        self.glyph = foreground.glyph;
    }
}

#[derive(Debug, Component)]
pub struct Building {}

impl Default for Building {
    fn default() -> Self {
        Self {}
    }
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum BuildingKind {
    Belt,
    Extractor,
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Resource {
    Coal,
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

impl Direction {
    pub fn rotate(&self) -> Self {
        match self {
            Self::Up => Self::Right,
            Self::Right => Self::Down,
            Self::Down => Self::Left,
            Self::Left => Self::Up,
        }
    }
}

#[derive(Component, Debug)]
pub struct ResourceMover {
    pub direction: Direction,
    pub payload: Option<Resource>,
}

impl Default for ResourceMover {
    fn default() -> Self {
        Self {
            direction: Direction::Right,
            payload: None,
        }
    }
}

#[derive(Component, Debug)]
pub struct Player {}

pub struct State {
    pub ecs: World,
    move_belt_resources: MoveResources,
}

impl Default for State {
    fn default() -> Self {
        Self {
            ecs: World::new(),
            move_belt_resources: MoveResources::default(),
        }
    }
}

fn try_move_player(delta_x: i32, delta_y: i32, ecs: &mut World) {
    let mut positions = ecs.write_storage::<Position>();
    let mut players = ecs.write_storage::<Player>();

    for (_player, position) in (&mut players, &mut positions).join() {
        position.x = clamp(position.x + delta_x, 0, MAP_WIDTH - 1);
        position.y = clamp(position.y + delta_y, 0, MAP_HEIGHT - 1);

        let mut player_position = ecs.write_resource::<Position>();
        player_position.x = position.x;
        player_position.y = position.y;
    }
}

fn try_build(ecs: &mut World, building_kind: BuildingKind) {
    let player_position = ecs
        .get_mut::<Position>()
        .expect("Player has no position.")
        .clone();

    // Do not build something if it already exists
    {
        let buildings = ecs.read_storage::<Building>();
        let positions = ecs.read_storage::<Position>();
        for (_building, position) in (&buildings, &positions).join() {
            if position == &player_position {
                return;
            }
        }
    }

    let builder = ecs
        .create_entity()
        .with(Building::default())
        .with(player_position);
    match building_kind {
        BuildingKind::Belt => builder
            .with(Renderable {
                glyph: rltk::to_cp437('v'),
                fg: RGB::named(rltk::YELLOW),
                bg: RGB::named(rltk::DARK_GREY),
            })
            .with(ResourceMover {
                direction: Direction::Down,
                payload: None,
            }),
        BuildingKind::Extractor => builder
            .with(Renderable {
                glyph: rltk::to_cp437('v'),
                fg: RGB::named(rltk::RED),
                bg: RGB::named(rltk::GREY30),
            })
            .with(ResourceMover {
                direction: Direction::Down,
                payload: None,
            }),
    }
    .build();
}

fn try_rotate_belt(ecs: &mut World) {
    let rotate_position = ecs
        .get_mut::<Position>()
        .expect("Player has no position.")
        .clone();

    let mut belts = ecs.write_storage::<ResourceMover>();
    let positions = ecs.read_storage::<Position>();
    let mut renderables = ecs.write_storage::<Renderable>();
    for (mut belt, position, renderable) in (&mut belts, &positions, &mut renderables).join() {
        if position == &rotate_position {
            belt.direction = belt.direction.rotate();
            renderable.merge_foreground((&*belt).into());
        }
    }
}

fn try_place_coal(ecs: &mut World) {
    let player_position = ecs
        .get_mut::<Position>()
        .expect("Player has no position.")
        .clone();

    let mut belts = ecs.write_storage::<ResourceMover>();
    let positions = ecs.read_storage::<Position>();
    let mut renderables = ecs.write_storage::<Renderable>();
    for (mut belt, position, renderable) in (&mut belts, &positions, &mut renderables).join() {
        if position == &player_position {
            belt.payload = Some(Resource::Coal);
            renderable.merge_foreground(Resource::Coal.into());
        }
    }
}

fn player_input(gs: &mut State, ctx: &mut Rltk) {
    // Player movement
    match ctx.key {
        // Nothing happened
        None => {}
        Some(key) => match key {
            // Movement
            VirtualKeyCode::H | VirtualKeyCode::Left => try_move_player(-1, 0, &mut gs.ecs),
            VirtualKeyCode::L | VirtualKeyCode::Right => try_move_player(1, 0, &mut gs.ecs),
            VirtualKeyCode::K | VirtualKeyCode::Up => try_move_player(0, -1, &mut gs.ecs),
            VirtualKeyCode::J | VirtualKeyCode::Down => try_move_player(0, 1, &mut gs.ecs),
            // Building things
            // Belts
            VirtualKeyCode::B => try_build(&mut gs.ecs, BuildingKind::Belt),
            VirtualKeyCode::R => try_rotate_belt(&mut gs.ecs),
            // Extractors
            VirtualKeyCode::E => try_build(&mut gs.ecs, BuildingKind::Extractor),

            // Place resources
            VirtualKeyCode::C => try_place_coal(&mut gs.ecs),

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

pub struct MoveResources {
    next_update: Instant,
}

impl Default for MoveResources {
    fn default() -> Self {
        Self {
            next_update: Instant::now(),
        }
    }
}

impl<'a> System<'a> for MoveResources {
    type SystemData = (
        ReadStorage<'a, Position>,
        WriteStorage<'a, ResourceMover>,
        WriteStorage<'a, Renderable>,
    );

    fn run(&mut self, (positions, mut belts, mut renderables): Self::SystemData) {
        let now = Instant::now();
        if self.next_update > now {
            return;
        }
        self.next_update = now + Duration::from_millis(500);

        let mut empty_positions: Vec<Position> = Vec::new();
        let mut target_position_to_source_positions: HashMap<Position, Vec<&Position>> =
            HashMap::new();

        let mut belts: HashMap<&Position, (&mut ResourceMover, &mut Renderable)> =
            (&positions, &mut belts, &mut renderables)
                .join()
                .map(|(position, belt, renderable)| {
                    if belt.payload.is_none() {
                        empty_positions.push(position.clone());
                    };
                    let mut target_position = position.clone();
                    match belt.direction {
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
                    (position, (belt, renderable))
                })
                .collect();

        for empty_position in empty_positions.iter() {
            let mut positions_to_visit = vec![empty_position];

            // Get the next position to visit
            while let Some(position) = positions_to_visit.pop() {
                // Get the data for this position, removing it from the map
                // as we only want to visit each belt once
                if let Some((belt, renderable)) = belts.remove(position) {
                    // If this position isn't empty, nothing to be done
                    if belt.payload.is_some() {
                        return;
                    }

                    // Get upstream positions
                    if let Some(source_positions) =
                        target_position_to_source_positions.get(position)
                    {
                        for source_position in source_positions.iter() {
                            if let Some((source_belt, source_renderable)) =
                                belts.get_mut(source_position)
                            {
                                // If the belt upstream doesn't have a payload
                                // try the next one
                                if source_belt.payload.is_none() {
                                    continue;
                                }
                                positions_to_visit.push(source_position);
                                std::mem::swap(&mut belt.payload, &mut source_belt.payload);
                                renderable.merge_foreground((&*belt).into());
                                source_renderable.merge_foreground((&**source_belt).into());
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
        self.move_belt_resources.run_now(&self.ecs);
        self.ecs.maintain();
    }
}
