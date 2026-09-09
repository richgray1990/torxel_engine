use bevy::prelude::*;
use crate::render::{CameraInput, MainCamera};

/// Система обработки ввода для движения камеры (стрелки клавиатуры)
pub fn handle_camera_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut camera_input: ResMut<CameraInput>,
) {
    camera_input.move_left = keyboard.pressed(KeyCode::ArrowLeft);
    camera_input.move_right = keyboard.pressed(KeyCode::ArrowRight);
    camera_input.move_up = keyboard.pressed(KeyCode::ArrowUp);
    camera_input.move_down = keyboard.pressed(KeyCode::ArrowDown);
}

/// Система сдвига мира на основе ввода камеры
/// Камера зафиксирована, мир сдвигается в противоположную сторону
pub fn move_world_by_camera(
    time: Res<Time>,
    camera_input: Res<CameraInput>,
    mut query: Query<&mut Transform, With<crate::render::WorldRoot>>,
) {
    const MOVE_SPEED: f32 = 10.0; // блоков в секунду
    
    for mut transform in query.iter_mut() {
        let mut offset = Vec3::ZERO;
        
        if camera_input.move_left {
            offset.x += MOVE_SPEED * time.delta_secs();
        }
        if camera_input.move_right {
            offset.x -= MOVE_SPEED * time.delta_secs();
        }
        if camera_input.move_up {
            offset.z -= MOVE_SPEED * time.delta_secs();
        }
        if camera_input.move_down {
            offset.z += MOVE_SPEED * time.delta_secs();
        }
        
        // Применяем сдвиг к миру (камера движется вправо -> мир влево)
        transform.translation += offset;
        
        // Здесь будет логика torus wrap:
        // Если transform.translation.x > 16.0 (размер чанка), то:
        // 1. Вычитаем 16.0 из translation
        // 2. Сдвигаем логические координаты чанков
        // 3. Пересоздаем/перемещаем Entity чанков
    }
}

/// Система настройки камеры при старте
pub fn setup_camera(
    mut commands: Commands,
) {
    // Ортогональная камера, повернутая на -35 градусов (смотрит вниз)
    commands.spawn((
        Camera2dBundle {
            projection: OrthographicProjection {
                scale: 1.0, // Будет настроено зумом позже
                ..default()
            },
            transform: Transform::from_xyz(0.0, 80.0, 0.0)
                .with_rotation(Quat::from_euler(EulerRot::XYZ, -35.0_f32.to_radians(), 0.0, 0.0)),
            ..default()
        },
        MainCamera,
    ));
}
