#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

use bevy::input::common_conditions::input_just_pressed;
use bevy::{prelude::*, render::primitives::Aabb, window::WindowResolution};
#[cfg(debug_assertions)]
use bevy_inspector_egui::bevy_egui::EguiPlugin;
#[cfg(debug_assertions)]
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use rand::Rng;

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
            primary_window: Window {
                resolution: WindowResolution::new(320., 512.),
                position: WindowPosition::Centered(MonitorSelection::Primary),
                title: "Doodle Jump".into(),
                resizable: false,
                ..default()
            }
            .into(),
            ..default()
        }),
    )
    .add_systems(Startup, setup)
    .add_systems(Update, (update_doodle, update_tiles, update_score))
    .add_systems(
        Update,
        trigger_restart.run_if(input_just_pressed(KeyCode::KeyR)),
    )
    .add_event::<RestartGame>()
    .add_observer(restart_game)
    .register_type::<NextTile>()
    .insert_resource(NextTile(rand::rng().random_range(90.0..140.)))
    .register_type::<Score>()
    .init_resource::<Score>();

    #[cfg(debug_assertions)]
    app.add_plugins((
        EguiPlugin {
            enable_multipass_for_primary_context: true,
        },
        WorldInspectorPlugin::new(),
    ))
    .add_systems(
        Update,
        (
            toggle_debug_boxes.run_if(input_just_pressed(KeyCode::Backquote)),
            toggle_gizmos,
        ),
    )
    .insert_resource(UiDebugOptions {
        enabled: true,
        ..default()
    });

    app.run();
}

#[cfg(debug_assertions)]
fn toggle_debug_boxes(mut options: ResMut<UiDebugOptions>) {
    options.toggle();
}

#[cfg(debug_assertions)]
fn toggle_gizmos(
    mut toggle: Local<bool>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut gizmos: Gizmos,
    objs: Query<(&Aabb, &Transform)>,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        *toggle ^= true;
    }
    if !*toggle {
        return;
    }

    for (a, t) in &objs {
        gizmos.rect_2d(
            Isometry2d::from_translation(t.translation.truncate()),
            Vec2::new(a.half_extents.x * 2., a.half_extents.y * 2.),
            Color::srgb(1., 0., 0.),
        );
    }
}

fn setup(mut cmd: Commands, assets: Res<AssetServer>) {
    cmd.spawn(Camera2d);

    cmd.spawn((
        Name::new("Background"),
        Sprite {
            image: assets.load("background.png"),
            ..default()
        },
        Transform::from_xyz(0., 0., -1.),
    ));

    cmd.spawn((
        Name::new("Score text"),
        ScoreText,
        Text::new("Score: 0"),
        TextColor(Color::BLACK),
        Transform::from_xyz(-90., 240., 0.),
    ));

    cmd.trigger(RestartGame);
}

#[derive(Component)]
struct DeathText;

#[derive(Event)]
struct RestartGame;

fn trigger_restart(mut cmd: Commands) {
    cmd.trigger(RestartGame);
}

fn restart_game(
    _: Trigger<RestartGame>,
    mut cmd: Commands,
    assets: Res<AssetServer>,
    mut score: ResMut<Score>,
    doodle: Query<Entity, With<Doodle>>,
    tiles: Query<Entity, With<Tile>>,
    mut score_text: Query<&mut Text, With<ScoreText>>,
    death_text: Query<Entity, With<DeathText>>,
) {
    if let Ok(e) = doodle.single() {
        cmd.entity(e).despawn();
    }
    for e in &tiles {
        cmd.entity(e).despawn();
    }

    cmd.spawn((
        Name::new("Doodle"),
        Doodle(0.),
        Sprite {
            image: assets.load("doodle.png"),
            ..default()
        },
        Transform::from_xyz(0., -195., 0.),
    ));

    // Tiles
    let mut height = -235.;
    let mut rng = rand::rng();
    cmd.spawn(tile(0., height, &assets));
    for _ in 0..3 {
        height += 120.;
        cmd.spawn(tile(
            rng.random_range(-TILE_BOUNDARY..=TILE_BOUNDARY),
            height,
            &assets,
        ));
    }

    score.0 = 0.;
    if let Ok(mut text) = score_text.single_mut() {
        text.0 = "Score: 0".into();
    }

    if let Ok(e) = death_text.single() {
        cmd.entity(e).despawn();
    }
}

#[derive(Component)]
struct Doodle(f32); // Vertical speed

fn update_doodle(
    mut doodle: Query<
        (Entity, &mut Transform, &mut Sprite, &mut Doodle, &Aabb),
        Without<Tile>,
    >,
    tiles: Query<(&Transform, &Aabb), With<Tile>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cmd: Commands,
) {
    let Ok((entity, mut transform, mut sprite, mut speed, aabb)) = doodle.single_mut() else {
        return;
    };

    // Horizontal movement
    let mut dir_x = 0.;
    const HORIZONTAL_VELOCITY: f32 = 500.;
    if keyboard.pressed(KeyCode::KeyA) {
        dir_x -= 1.;
        sprite.flip_x = false;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        dir_x += 1.;
        sprite.flip_x = true;
    }
    transform.translation.x += dir_x * HORIZONTAL_VELOCITY * time.delta_secs();

    // Wrap around
    const HALF_WIDTH: f32 = 160.;
    let x = transform.translation.x;
    if x > HALF_WIDTH {
        transform.translation.x = -HALF_WIDTH;
    } else if x < -HALF_WIDTH {
        transform.translation.x = HALF_WIDTH;
    }

    // Vertical movement
    const GRAVITY: f32 = -900.;
    speed.0 += GRAVITY * time.delta_secs();
    transform.translation.y += speed.0 * time.delta_secs();
    if transform.translation.y >= 0. {
        transform.translation.y = 0.;
    }

    if transform.translation.y < -256. {
        cmd.entity(entity).despawn();

        cmd.spawn((
            Name::new("Death text"),
            DeathText,
            Text2d::new("You died.\nPress `R` to restart."),
            TextColor(Color::srgb(1., 0., 0.)),
        ));
    }

    // Collisions
    if speed.0 > 0. {
        return;
    }
    for (tile_transform, tile_aabb) in &tiles {
        let x_diff = transform.translation.x - tile_transform.translation.x;
        let y_diff = transform.translation.y - tile_transform.translation.y;
        if y_diff > (aabb.half_extents.y + tile_aabb.half_extents.y) - 15.
            && y_diff < (aabb.half_extents.y + tile_aabb.half_extents.y)
            && x_diff.abs() < (aabb.half_extents.x / 2. + tile_aabb.half_extents.x)
        {
            speed.0 = 500.;
        }
    }
}

#[derive(Component)]
struct Tile;

fn tile(x: f32, y: f32, assets: &AssetServer) -> impl Bundle {
    (
        Name::new("Tile"),
        Tile,
        Sprite {
            image: assets.load("tile.png"),
            ..default()
        },
        Transform::from_xyz(x, y, -0.5),
    )
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
struct NextTile(f32); // Distance to next tile

const NEXT_TILE_Y: f32 = 266.;
const TILE_BOUNDARY: f32 = 130.;

fn update_tiles(
    mut tiles: Query<(Entity, &mut Transform), With<Tile>>,
    doodle: Query<(&Doodle, &Transform), Without<Tile>>,
    time: Res<Time>,
    mut cmd: Commands,
    assets: Res<AssetServer>,
    mut next_tile: ResMut<NextTile>,
) {
    let Ok((doodle_speed, doodle_transform)) = doodle.single() else {
        return;
    };
    if doodle_transform.translation.y < -5. || doodle_speed.0 < 0. {
        return;
    }

    // Move tiles
    let mut max_height = -256.0;
    for (e, mut t) in &mut tiles {
        let res = t.translation.y - doodle_speed.0 * time.delta_secs();
        if res < -265. {
            cmd.entity(e).despawn();
            continue;
        }

        if res > max_height {
            max_height = res;
        }
        t.translation.y = res;
    }

    // Spawn next tile
    let height_diff = NEXT_TILE_Y - max_height;
    if height_diff >= next_tile.0 {
        let mut rng = rand::rng();
        cmd.spawn(tile(
            rng.random_range(-TILE_BOUNDARY..=TILE_BOUNDARY),
            NEXT_TILE_Y,
            &assets,
        ));
        next_tile.0 = rng.random_range(90.0..=140.);
    }
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct Score(f32);

#[derive(Component)]
struct ScoreText;

fn update_score(
    doodle: Query<(&Doodle, &Transform)>,
    mut score: ResMut<Score>,
    mut score_text: Query<&mut Text, With<ScoreText>>,
) {
    let Ok((doodle_speed, doodle_transform)) = doodle.single() else {
        return;
    };
    if doodle_transform.translation.y < -5. || doodle_speed.0 < 0. {
        return;
    }

    score.0 += doodle_speed.0 / 200.;
    let mut score_text = score_text.single_mut().unwrap();
    score_text.0 = format!("Score: {:.0}", score.0);
}
