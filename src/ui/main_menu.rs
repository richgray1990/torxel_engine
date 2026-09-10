use bevy::prelude::*;
use crate::game_state::GameState;

/// Маркер для корневой ноды главного меню
#[derive(Component)]
pub struct MainMenuRoot;

/// Маркер для кнопки "Играть"
#[derive(Component)]
pub struct PlayButton;

/// Маркер для кнопки "Выход"
#[derive(Component)]
pub struct ExitButton;

/// Система для создания главного меню
pub fn setup_main_menu(mut commands: Commands) {
    // Корневой узел UI (центрирование по вертикали и горизонтали)
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            ..default()
        },
        MainMenuRoot,
    )).with_children(|parent| {
        // Заголовок
        parent.spawn((
            TextBundle::from_section(
                "Voxel World Engine",
                TextStyle {
                    font_size: 48.0,
                    color: Color::WHITE,
                    ..default()
                },
            ),
        ));

        // Кнопка "Играть"
        parent.spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(200.0),
                    height: Val::Px(50.0),
                    border: UiRect::all(Val::Px(2.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                border_color: BorderColor(Color::GRAY),
                background_color: BackgroundColor(Color::DARK_GRAY),
                ..default()
            },
            PlayButton,
        )).with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Играть",
                TextStyle {
                    font_size: 24.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
        });

        // Кнопка "Выход"
        parent.spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(200.0),
                    height: Val::Px(50.0),
                    border: UiRect::all(Val::Px(2.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                border_color: BorderColor(Color::GRAY),
                background_color: BackgroundColor(Color::DARK_GRAY),
                ..default()
            },
            ExitButton,
        )).with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Выход",
                TextStyle {
                    font_size: 24.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
        });
    });
}

/// Система обработки нажатий кнопок меню
pub fn handle_menu_actions(
    mut next_state: ResMut<NextState<GameState>>,
    mut app_exit_events: EventWriter<AppExit>,
    play_interactions: Query<&Interaction, (Changed<Interaction>, With<PlayButton>)>,
    exit_interactions: Query<&Interaction, (Changed<Interaction>, With<ExitButton>)>,
) {
    for interaction in &play_interactions {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::Playing);
        }
    }

    for interaction in &exit_interactions {
        if *interaction == Interaction::Pressed {
            app_exit_events.send(AppExit::Success);
        }
    }
}

/// Система удаления главного меню при выходе из состояния MainMenu
pub fn despawn_main_menu(
    mut commands: Commands,
    menu_query: Query<Entity, Or<(With<MainMenuRoot>, With<PlayButton>, With<ExitButton>)>>,
) {
    for entity in &menu_query {
        commands.entity(entity).despawn_recursive();
    }
}
