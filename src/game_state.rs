use bevy::prelude::*;

/// Состояния игры
#[derive(States, Debug, Hash, PartialEq, Eq, Clone, Default)]
pub enum GameState {
    #[default]
    MainMenu,
    Playing,
}
