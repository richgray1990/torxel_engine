//! Главное меню на bevy_egui
//! 
//! Функционал:
//! - Создание нового мира (ввод сида, параметров)
//! - Список сохранений с загрузкой/удалением
//! - Настройки (будущее расширение)

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use bevy::app::States;
use bevy::log::info;

use crate::core::ChunkManager;
use crate::generation::{GenerationParams, TerrainGenerator};
use crate::serialization::{SaveHeader, get_saves_directory, save_world, load_world, list_saves};

/// Состояния приложения
#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    Generating,
    Playing,
    Loading,
}

/// Ресурс для хранения состояния UI главного меню
#[derive(Resource, Default)]
pub struct MainMenuState {
    /// Введённый пользователем сид
    pub seed_input: String,
    
    /// Имя нового мира
    pub world_name_input: String,
    
    /// Выбранный размер мира в чанках (8, 16, 32, 64, 128, 256, 512)
    pub world_size_chunks_selection: usize,
    
    /// Ошибка генерации/загрузки (если есть)
    pub error_message: Option<String>,
    
    /// Список сохранений для отображения
    pub saved_games: Vec<SaveHeader>,
    
    /// Индекс выбранного сохранения
    pub selected_save_index: Option<usize>,
}

/// Компонент для标记 сущности главного меню
#[derive(Component)]
pub struct MainMenuUI;

/// Плагин главного меню
pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_state::<AppState>()
            .init_resource::<MainMenuState>()
            .add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
            .add_systems(
                Update,
                (
                    main_menu_ui.run_if(in_state(AppState::MainMenu)),
                    show_loading_screen.run_if(in_state(AppState::Loading)),
                )
            )
            .add_systems(OnExit(AppState::MainMenu), cleanup_main_menu);
    }
}

/// Настроить UI главного меню
fn setup_main_menu(mut commands: Commands) {
    // Главный UI будет рендериться через bevy_egui, поэтому не создаём 2D сущности
    // Только маркер для отладки
    commands.spawn((
        MainMenuUI,
        Name::new("MainMenuUI"),
    ));
}

/// Очистить UI главного меню при выходе
fn cleanup_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenuUI>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

/// Главная функция UI меню
fn main_menu_ui(
    mut contexts: EguiContexts,
    mut state: ResMut<MainMenuState>,
    mut next_state: ResMut<NextState<AppState>>,
    mut chunk_manager: Option<ResMut<ChunkManager>>,
) {
    let ctx = contexts.ctx_mut();
    
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            
            // Заголовок
            ui.heading(egui::RichText::new("Torxel World").size(40.0));
            ui.label("Высокопроизводительный воксельный движок с физикой DHIMMS");
            
            ui.add_space(30.0);
            
            // Кнопка "Новый мир"
            ui.collapsing("🌍 Новый мир", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Название:");
                    ui.text_edit_singleline(&mut state.world_name_input);
                });
                
                ui.horizontal(|ui| {
                    ui.label("Сид:");
                    ui.text_edit_singleline(&mut state.seed_input);
                });
                
                ui.label("Размер мира (чанков по XZ, округляется до кратного 16):");
                ui.horizontal(|ui| {
                    if ui.radio_value(&mut state.world_size_chunks_selection, 8, "8×8 (мини)").clicked() {}
                    if ui.radio_value(&mut state.world_size_chunks_selection, 16, "16×16 (стандарт)").clicked() {}
                    if ui.radio_value(&mut state.world_size_chunks_selection, 32, "32×32 (большой)").clicked() {}
                    if ui.radio_value(&mut state.world_size_chunks_selection, 64, "64×64 (огромный)").clicked() {}
                    if ui.radio_value(&mut state.world_size_chunks_selection, 128, "128×128 (гигантский)").clicked() {}
                    if ui.radio_value(&mut state.world_size_chunks_selection, 256, "256×256 (экстремальный)").clicked() {}
                    if ui.radio_value(&mut state.world_size_chunks_selection, 512, "512×512 (максимум)").clicked() {}
                });
                
                ui.add_space(10.0);
                
                // Показать размер в блоках
                let blocks_per_chunk = 16; // CHUNK_SIZE_XZ
                let total_blocks = state.world_size_chunks_selection * blocks_per_chunk;
                ui.label(format!("Всего блоков: {}×{} = {}", 
                    total_blocks, total_blocks, total_blocks * total_blocks));
                
                ui.add_space(10.0);
                
                if ui.button("✨ Создать мир").clicked() {
                    // Парсить сид или использовать случайный
                    let seed: u64 = state.seed_input.parse()
                        .unwrap_or_else(|_| rand::random());
                    
                    // Сохранить сид в input для отображения
                    state.seed_input = seed.to_string();
                    
                    // Перейти к генерации
                    next_state.set(AppState::Generating);
                    
                    // Здесь будет логика генерации мира
                    // Для v1 просто переходим в Playing
                    info!("Creating new world with seed: {}", seed);
                    
                    // В реальной реализации здесь вызывается генератор
                    // и инициализируется ChunkManager
                    
                    next_state.set(AppState::Playing);
                }
            });
            
            ui.add_space(20.0);
            
            // Кнопка "Загрузить мир"
            ui.collapsing("📁 Загрузить мир", |ui| {
                // Обновить список сохранений
                if ui.button("🔄 Обновить список").clicked() {
                    match list_saves(get_saves_directory()) {
                        Ok(saves) => state.saved_games = saves,
                        Err(e) => state.error_message = Some(format!("Ошибка загрузки списка: {}", e)),
                    }
                }
                
                ui.add_space(10.0);
                
                if state.saved_games.is_empty() {
                    ui.label("Нет сохранений");
                } else {
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for (i, save) in state.saved_games.iter().enumerate() {
                            let is_selected = state.selected_save_index == Some(i);
                            
                            ui.selectable_value(&mut state.selected_save_index, Some(i), 
                                format!("{} (Seed: {}, Date: {})", 
                                    save.name, 
                                    save.seed,
                                    format_timestamp(save.modified_at)
                                )
                            );
                        }
                    });
                    
                    ui.add_space(10.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("📂 Загрузить").clicked() {
                            if let Some(idx) = state.selected_save_index {
                                // Найти файл сохранения и загрузить
                                // Для v1 просто переходим в Playing
                                info!("Loading save index: {}", idx);
                                next_state.set(AppState::Playing);
                            }
                        }
                        
                        if ui.button("🗑️ Удалить").clicked() {
                            if let Some(idx) = state.selected_save_index {
                                // Удалить сохранение
                                state.saved_games.remove(idx);
                                state.selected_save_index = None;
                            }
                        }
                    });
                }
            });
            
            ui.add_space(20.0);
            
            // Кнопка "Настройки" (заглушка для будущего)
            if ui.button("⚙️ Настройки").clicked() {
                // TODO: открыть окно настроек
            }
            
            ui.add_space(20.0);
            
            // Кнопка выхода
            if ui.button("🚪 Выход").clicked() {
                std::process::exit(0);
            }
            
            // Показать ошибку если есть
            if let Some(ref error) = state.error_message {
                ui.add_space(20.0);
                ui.colored_label(egui::Color32::RED, format!("❌ {}", error));
                
                if ui.button("OK").clicked() {
                    state.error_message = None;
                }
            }
        });
    });
}

/// Экран загрузки
fn show_loading_screen(mut contexts: EguiContexts) {
    let ctx = contexts.ctx_mut();
    
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("Генерация мира...");
            
            ui.add_space(20.0);
            
            // Прогресс бар (пока фиктивный)
            ui.label("Пожалуйста, подождите");
            
            // Анимация загрузки
            ui.spinner();
        });
    });
}

/// Форматировать timestamp в читаемую дату
fn format_timestamp(timestamp: u64) -> String {
    // Простая реализация, в реальном проекте использовать chrono
    format!("{}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_state_default() {
        let state = MainMenuState::default();
        assert!(state.seed_input.is_empty());
        assert!(state.saved_games.is_empty());
    }
}
