use bevy::prelude::*;
use crate::core::{Chunk, ChunkManager, Cell};
use crate::render::{ChunkMeshes, ChunkPos, SolidMaterial, TextureAssets};

/// Система генерации мешей для чанков
/// Вызывается когда чанк помечен как dirty или при первом создании
pub fn generate_chunk_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<Assets<SolidMaterial>>,
    textures: Res<TextureAssets>,
    chunk_manager: Res<ChunkManager>,
    query: Query<(Entity, &ChunkPos, Option<&ChunkMeshes>), Without<Chunk>>,
) {
    // Проходим по всем чанкам в менеджере
    // В реальной реализации здесь будет итерация по активным чанкам
    // Для примера - заглушка логики
    
    // Логика:
    // 1. Для каждого чанка создаем два меша: Solid и Water
    // 2. Проходим по всем ячейкам чанка (x, y, z)
    // 3. Применяем face culling:
    //    - Air пропускаем
    //    - Water: рисуем топ если сверху Air, стены если сосед Air
    //    - Solid: рисуем топ если сверху Air или Water, стены если сосед Air или Water
    // 4. Для каждой грани вычисляем neighbor_mask (4 бита для боковых соседей)
    // 5. Сохраняем mask в атрибут вершины (например, в UV.z или Color.a)
    
    // Пример создания простого меша (заглушка)
    /*
    for (pos, chunk) in chunk_manager.iter_chunks() {
        let solid_mesh = create_solid_mesh(chunk, &textures);
        let water_mesh = create_water_mesh(chunk, &textures);
        
        if let Some((entity, _chunk_pos, existing_meshes)) = query.iter().find(...) {
            // Обновляем существующий меш
        } else {
            // Создаем новый entity для чанка
            commands.spawn((
                ChunkPos { x: pos.x, z: pos.z },
                ChunkMeshes {
                    solid_mesh: Some(meshes.add(solid_mesh)),
                    water_mesh: Some(meshes.add(water_mesh)),
                },
                // Transform будет установлен системой синхронизации
            ));
        }
    }
    */
}

/// Создание меша для твердых блоков (земля, камень, магма)
fn create_solid_mesh(chunk: &Chunk, textures: &TextureAssets) -> Mesh {
    let mut mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList, default());
    
    // Здесь будет логика прохода по ячейкам и генерации вершин
    // С сохранением neighbor_mask в атрибуты
    
    mesh
}

/// Создание меша для воды
fn create_water_mesh(chunk: &Chunk, textures: &TextureAssets) -> Mesh {
    let mut mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList, default());
    
    // Логика генерации только верхних и боковых граней воды
    // Без обводки (или с простой обводкой)
    
    mesh
}
