//! Voxel World Engine - Высокопроизводительный воксельный движок
//! 
//! Архитектурные принципы:
//! - Zero-Allocation Core: данные в плоских Vec<Cell>
//! - ECS (Bevy): логика разделена на системы
//! - Toroidal wrapping: циклическая топология мира
//! - Безопасность: #[repr(C)], квантование, битовые флаги

use bevy::prelude::*;
use bevy::log::{info, LogPlugin};

// Модули движка
mod core;
mod utils;
mod generation;
mod serialization;
mod render;

use core::{ChunkManager, Cell, Material};

fn main() {
    App::new()
        // Основные плагины Bevy
        .add_plugins(DefaultPlugins.set(LogPlugin {
            filter: "torxel_engine=info".to_string(),
            level: bevy::log::Level::INFO,
            custom_layer: |_| None,
            fmt_layer: |_| None,
        }))

        // Инициализация ресурсов
        .init_resource::<ChunkManager>()
        
        // Системы
        .add_systems(Startup, setup_camera)
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
