use rltk::{self, RGB};
use specs::prelude::*;

use factermio_core::{
    Building, Direction, Map, Player, Position, Renderable, Resource, ResourceBuffer,
    ResourceExtractor, ResourceMover, State,
};

fn main() {
    use rltk::RltkBuilder;
    let context = RltkBuilder::simple80x50()
        .with_title("Factermio")
        .with_tile_dimensions(16, 16)
        .build();
    let mut gs = State::default();
    gs.ecs.register::<Position>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<ResourceBuffer>();
    gs.ecs.register::<ResourceExtractor>();
    gs.ecs.register::<Player>();
    gs.ecs.register::<ResourceMover>();
    gs.ecs.register::<Building>();

    gs.ecs.insert(Map::default());
    gs.ecs.insert(Position { x: 40, y: 25 });

    gs.ecs
        .create_entity()
        .with(Position { x: 40, y: 25 })
        .with(Renderable {
            glyph: rltk::to_cp437('@'),
            fg: RGB::named(rltk::GREEN),
            bg: RGB::named(rltk::BLACK),
        })
        .with(Player {})
        .build();

    for i in 0..10 {
        gs.ecs
            .create_entity()
            .with(Position { x: i * 7, y: 20 })
            .with(Renderable {
                glyph: rltk::to_cp437('x'),
                fg: RGB::named(rltk::RED1),
                bg: RGB::named(rltk::BLACK),
            })
            .with(ResourceBuffer {
                resource: Resource::Coal,
                remaining: 1000,
            })
            .build();
    }

    gs.ecs
        .create_entity()
        .with(Position { x: 10, y: 19 })
        .with(Renderable {
            glyph: rltk::to_cp437('c'),
            fg: RGB::named(rltk::YELLOW),
            bg: RGB::named(rltk::DARK_GREY),
        })
        .with(Building::default())
        .with(ResourceMover {
            direction: Direction::Down,
            payload: Some(Resource::Coal),
        })
        .build();
    for y in 20..24 {
        gs.ecs
            .create_entity()
            .with(Position { x: 10, y })
            .with(Renderable {
                glyph: rltk::to_cp437('v'),
                fg: RGB::named(rltk::YELLOW),
                bg: RGB::named(rltk::DARK_GREY),
            })
            .with(Building::default())
            .with(ResourceMover {
                direction: Direction::Down,
                payload: None,
            })
            .build();
    }
    for x in 8..14 {
        gs.ecs
            .create_entity()
            .with(Position { x, y: 24 })
            .with(Renderable {
                glyph: rltk::to_cp437('>'),
                fg: RGB::named(rltk::YELLOW),
                bg: RGB::named(rltk::DARK_GREY),
            })
            .with(Building::default())
            .with(ResourceMover {
                direction: Direction::Right,
                payload: None,
            })
            .build();
    }

    rltk::main_loop(context, gs);
}
