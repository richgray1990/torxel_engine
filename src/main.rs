//! Voxel World Engine - Высокопроизводительный воксельный движок
//! 
//! Архитектурные принципы:
//! - Zero-Allocation Core: данные в плоских Vec<Cell>
//! - ECS (Bevy): логика разделена на системы
//! - Toroidal wrapping: циклическая топология мира
//! - Безопасность: #[repr(C)], квантование, битовые флаги

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy::log::{info, LogPlugin};

// Модули движка
mod core;
mod utils;
mod generation;
mod serialization;
mod ui;
mod render;

use core::{ChunkManager, Cell, Material};
use ui::{MainMenuPlugin, AppState, MainMenuState};

fn main() {
    App::new()
        // Основные плагины Bevy
        .add_plugins(DefaultPlugins.set(LogPlugin {
            level: bevy::log::Level::INFO,
            filter: "torxel_engine=info".to_string(),
            custom_layer: None,
        }))
        
        // Плагин egui для UI
        .add_plugins(EguiPlugin)
        
        // Плагин главного меню
        .add_plugins(MainMenuPlugin)
        
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
            info!("Размер чанка: {}x{}x{} ячеек", core::CHUNK_SIZE, core::CHUNK_SIZE, core::CHUNK_SIZE);
            info!("Размер мира: {}x{}x{} чанков", core::WORLD_SIZE_CHUNKS, core::WORLD_SIZE_CHUNKS, core::WORLD_SIZE_CHUNKS);
            info!("Общий размер мира: {}x{}x{} ячеек", core::WORLD_SIZE_CELLS, core::WORLD_SIZE_CELLS, core::WORLD_SIZE_CELLS);
            info!("Размер Cell: {} байт", core::CELL_SIZE);
            
            // Тестовая генерация нескольких чанков
            test_chunk_generation(&mut chunk_manager);
            
            SHOWN_INFO = true;
        }
    }
    
    // Вернуться в меню через несколько кадров (для демонстрации)
    // В реальной игре здесь будет геймплей
}

/// Тестовая генерация чанков для проверки ядра
fn test_chunk_generation(chunk_manager: &mut ChunkManager) {
    use generation::{TerrainGenerator, GenerationParams};
    
    let params = GenerationParams {
        seed: 42,
        world_size_chunks: core::WORLD_SIZE_CHUNKS,
        ..Default::default()
    };
    
    let generator = TerrainGenerator::new(params);
    
    info!("Генерация тестовых чанков...");
    
    // Сгенерировать несколько чанков для теста
    let test_positions = [
        (0, 0, 0),
        (1, 0, 0),
        (0, 1, 0),
        (0, 0, 1),
    ];
    
    for (x, y, z) in test_positions.iter() {
        let chunk_pos = core::ChunkPos::new(*x, *y, *z);
        let chunk = generator.generate_chunk(chunk_pos);
        
        // Вставить чанк в менеджер (через установку ячейки)
        let global_x = x * core::CHUNK_SIZE as i32;
        let global_y = y * core::CHUNK_SIZE as i32;
        let global_z = z * core::CHUNK_SIZE as i32;
        
        // Просто убедиться, что чанк создаётся
        let _cell = chunk_manager.get_cell(global_x, global_y, global_z);
    }
    
    info!("Создано чанков: {}", chunk_manager.active_chunk_count());
    info!("Ячеек доступно: {}", chunk_manager.stats().cells_accessed);
}