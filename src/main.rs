//! Voxel World Engine - Высокопроизводительный воксельный движок
//! 
//! Архитектурные принципы:
//! - Zero-Allocation Core: данные в плоских Vec<Cell>
//! - ECS (Bevy): логика разделена на системы
//! - Toroidal wrapping: циклическая топология мира
//! - Безопасность: #[repr(C)], квантование, битовые флаги

use bevy::prelude::*;
use bevy::log::{info, LogPlugin};
use bevy::state::app::StatesPlugin;

// Модули движка
mod core;
mod utils;
mod generation;
mod serialization;
mod render;
mod ui;
mod game_state;

use core::{ChunkManager, Cell, Material};
use game_state::GameState;
use ui::*;

fn main() {
    App::new()
        // Основные плагины Bevy
        .add_plugins(DefaultPlugins.set(LogPlugin {
            filter: "torxel_engine=info".to_string(),
            level: bevy::log::Level::INFO,
            custom_layer: |_| None,
            fmt_layer: |_| None,
        }))
        // Плагин состояний
        //.add_plugins(StatesPlugin)
        // Инициализация состояний игры
        .init_state::<GameState>()
        // Инициализация ресурсов
        .init_resource::<ChunkManager>()
        
        // Системы
        .add_systems(Startup, setup_camera)
        // Системы главного меню
        .add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
        .add_systems(Update, handle_menu_actions.run_if(in_state(GameState::MainMenu)))
        .add_systems(OnExit(GameState::MainMenu), despawn_main_menu)
        .run();
}

/// Настроить камеру для просмотра мира
fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(100.0, 100.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
        Name::new("MainCamera"),
    ));
    
    info!("Camera initialized");
}
