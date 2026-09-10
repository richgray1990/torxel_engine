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
        .add_systems(Update, handle_playing_state.run_if(in_state(AppState::Playing)))
        
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

/// Обработка состояния игры (заглушка для v1)
fn handle_playing_state(
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkManager>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    // В v1 просто показываем сообщение и возвращаемся в меню
    // В v2 здесь будет основная симуляция
    
    static mut SHOWN_INFO: bool = false;
    
    unsafe {
        if !SHOWN_INFO {
            info!("=== Voxel World v1 - Ядро ===");
            info!("Мир использует торическую топологию (без границ)");
            info!("Размер чанка: {}x{}x64 ячеек", core::CHUNK_SIZE_XZ, core::CHUNK_SIZE_XZ);
            info!("Размер мира: {}x{} чанков", core::WORLD_SIZE_CHUNKS_XZ, core::WORLD_SIZE_CHUNKS_XZ);
            info!("Общий размер мира: {}x{}x64 ячеек", core::WORLD_SIZE_CELLS_XZ, core::WORLD_SIZE_CELLS_XZ);
            
            // Тестовая генерация нескольких чанков
            
            
            SHOWN_INFO = true;
        }
    }
    
    // Вернуться в меню через несколько кадров (для демонстрации)
    // В реальной игре здесь будет геймплей
}