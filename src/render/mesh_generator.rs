use bevy::prelude::*;
use crate::core::{Chunk, ChunkManager, Cell, CHUNK_SIZE_XZ, CHUNK_SIZE_Y};
use crate::render::{ChunkMeshes, ChunkPos, TextureAssets};

/// Константы для битовой маски соседей
const NEIGHBOR_AIR: u8 = 0b0001; // Бит 0: сосед справа (East) Air
const NEIGHBOR_SOLID: u8 = 0b0010; // Бит 1: сосед слева (West) Air
const NEIGHBOR_FRONT_AIR: u8 = 0b0100; // Бит 2: сосед спереди (South) Air
const NEIGHBOR_BACK_AIR: u8 = 0b1000; // Бит 3: сосед сзади (North) Air

/// Система генерации мешей для чанков
pub fn generate_chunk_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_manager: Res<ChunkManager>,
    textures: Res<TextureAssets>,
    query: Query<(Entity, &ChunkPos), With<ChunkMeshes>>,
    dirty_chunks: Query<&ChunkPos, Changed<Chunk>>, // Упрощенно: считаем чанк dirty если он изменился
) {
    // Собираем список чанков для пересчетамешей
    let mut chunks_to_update = Vec::new();
    
    // Проверяем все активные чанки в менеджере
    // В реальной реализации нужен итератор по активным чанкам
    // Здесь заглушка - в будущем будет итерация по HashMap
    
    // Для примера: если есть измененные чанки или новые
    for chunk_pos in dirty_chunks.iter() {
        chunks_to_update.push(*chunk_pos);
    }
    
    // Если нет измененных, но есть чанки в менеджере - генерируем для всех
    if chunks_to_update.is_empty() {
        // Заглушка: итерация по всем чанкам
        // chunk_manager.chunks.iter().for_each(|(pos, chunk)| { ... });
    }
    
    for chunk_pos in chunks_to_update {
        // Получаем чанк из менеджера
        if let Some(chunk) = chunk_manager.get_chunk(chunk_pos.x, chunk_pos.z) {
            // Генерируем меши
            let solid_mesh = create_solid_mesh(chunk, &textures);
            let water_mesh = create_water_mesh(chunk, &textures);
            
            // Находим существующий entity или создаем новый
            let mut found = false;
            for (entity, pos, mut mesh_comp) in query.iter() {
                if *pos == chunk_pos {
                    // Обновляем меши
                    // TODO: обновление мешей через ResMut<Assets<Mesh>>
                    found = true;
                    break;
                }
            }
            
            if !found {
                // Создаем новый entity для чанка
                commands.spawn((
                    chunk_pos,
                    ChunkMeshes {
                        solid_mesh: Some(meshes.add(solid_mesh)),
                        water_mesh: Some(meshes.add(water_mesh)),
                    },
                    Transform::from_xyz(
                        chunk_pos.x as f32 * CHUNK_SIZE_XZ as f32,
                        0.0,
                        chunk_pos.z as f32 * CHUNK_SIZE_XZ as f32,
                    ),
                    Visibility::default(),
                ));
            }
        }
    }
}

/// Создание меша для твердых блоков (земля, камень, магма)
fn create_solid_mesh(chunk: &Chunk, textures: &TextureAssets) -> Mesh {
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut uvs = Vec::<[f32; 2]>::new();
    let mut neighbor_masks = Vec::<[f32; 4]>::new(); // RGBA, используем R для маски
    
    // Проход по всем ячейкам чанка
    for y in 0..CHUNK_SIZE_Y {
        for z in 0..CHUNK_SIZE_XZ {
            for x in 0..CHUNK_SIZE_XZ {
                let cell = chunk.get(x as u8, y as u8, z as u8);
                
                // Пропускаем воздух и воду
                if cell.is_air() || cell.is_water() {
                    continue;
                }
                
                // Проверяем соседей для face culling
                let top_air = is_air_or_water(chunk, x, y + 1, z);
                let bottom_air = is_air_or_water(chunk, x, y.wrapping_sub(1), z);
                let right_air = is_air_or_water(chunk, x + 1, y, z);
                let left_air = is_air_or_water(chunk, x.wrapping_sub(1), y, z);
                let front_air = is_air_or_water(chunk, x, y, z + 1);
                let back_air = is_air_or_water(chunk, x, y, z.wrapping_sub(1));
                
                // Вычисляем маску соседей (4 бита для боковых граней)
                let mut mask: u8 = 0;
                if right_air { mask |= 0b0001; } // East
                if left_air { mask |= 0b0010; }  // West
                if front_air { mask |= 0b0100; } // South
                if back_air { mask |= 0b1000; }  // North
                
                // Конвертируем маску в f32 для атрибута вершины (0.0 или 1.0)
                let mask_f = mask as f32 / 15.0; // Нормализуем 0-15 -> 0.0-1.0
                
                // Рисуем топ грань если сверху Air или Water
                if top_air {
                    add_top_face(&mut positions, &mut normals, &mut uvs, &mut neighbor_masks, 
                                 x as f32, y as f32, z as f32, mask_f);
                }
                
                // Рисуем боковые грани если сосед Air или Water
                if right_air {
                    add_right_face(&mut positions, &mut normals, &mut uvs, &mut neighbor_masks,
                                   x as f32, y as f32, z as f32, mask_f);
                }
                if left_air {
                    add_left_face(&mut positions, &mut normals, &mut uvs, &mut neighbor_masks,
                                  x as f32, y as f32, z as f32, mask_f);
                }
                if front_air {
                    add_front_face(&mut positions, &mut normals, &mut uvs, &mut neighbor_masks,
                                   x as f32, y as f32, z as f32, mask_f);
                }
                if back_air {
                    add_back_face(&mut positions, &mut normals, &mut uvs, &mut neighbor_masks,
                                  x as f32, y as f32, z as f32, mask_f);
                }
            }
        }
    }
    
    // Создаем меш
    let mut mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, neighbor_masks); // Используем Color для передачи маски
    
    mesh
}

/// Создание меша для воды
fn create_water_mesh(chunk: &Chunk, textures: &TextureAssets) -> Mesh {
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut uvs = Vec::<[f32; 2]>::new();
    let mut colors = Vec::<[f32; 4]>::new(); // Цвет для прозрачности
    
    // Проход по всем ячейкам чанка
    for y in 0..CHUNK_SIZE_Y {
        for z in 0..CHUNK_SIZE_XZ {
            for x in 0..CHUNK_SIZE_XZ {
                let cell = chunk.get(x as u8, y as u8, z as u8);
                
                // Пропускаем не-воду
                if !cell.is_water() {
                    continue;
                }
                
                // Проверяем соседей
                let top_air = is_air_only(chunk, x, y + 1, z); // Только Air, не Solid
                let right_air = is_air_only(chunk, x + 1, y, z);
                let left_air = is_air_only(chunk, x.wrapping_sub(1), y, z);
                let front_air = is_air_only(chunk, x, y, z + 1);
                let back_air = is_air_only(chunk, x, y, z.wrapping_sub(1));
                
                // Рисуем топ грань только если сверху Air
                if top_air {
                    add_top_face_water(&mut positions, &mut normals, &mut uvs, &mut colors,
                                       x as f32, y as f32, z as f32);
                }
                
                // Рисуем боковые грани только если сосед Air
                if right_air {
                    add_right_face_water(&mut positions, &mut normals, &mut uvs, &mut colors,
                                         x as f32, y as f32, z as f32);
                }
                if left_air {
                    add_left_face_water(&mut positions, &mut normals, &mut uvs, &mut colors,
                                        x as f32, y as f32, z as f32);
                }
                if front_air {
                    add_front_face_water(&mut positions, &mut normals, &mut uvs, &mut colors,
                                         x as f32, y as f32, z as f32);
                }
                if back_air {
                    add_back_face_water(&mut positions, &mut normals, &mut uvs, &mut colors,
                                        x as f32, y as f32, z as f32);
                }
            }
        }
    }
    
    let mut mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    
    mesh
}

/// Проверка: сосед Air или Water (для Solid)
fn is_air_or_water(chunk: &Chunk, x: usize, y: usize, z: usize) -> bool {
    // Обработка границ чанка - упрощенно считаем Air
    if x >= CHUNK_SIZE_XZ || y >= CHUNK_SIZE_Y || z >= CHUNK_SIZE_XZ {
        return true; // Считаем Air для границ (в будущем проверка соседей из других чанков)
    }
    
    let cell = chunk.get(x as u8, y as u8, z as u8);
    cell.is_air() || cell.is_water()
}

/// Проверка: только Air (для Water)
fn is_air_only(chunk: &Chunk, x: usize, y: usize, z: usize) -> bool {
    if x >= CHUNK_SIZE_XZ || y >= CHUNK_SIZE_Y || z >= CHUNK_SIZE_XZ {
        return true;
    }
    
    let cell = chunk.get(x as u8, y as u8, z as u8);
    cell.is_air()
}

/// Добавление топ грани для Solid
fn add_top_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    masks: &mut Vec<[f32; 4]>,
    x: f32, y: f32, z: f32, mask: f32,
) {
    // Вершины квадрата (top face)
    positions.push([x, y + 1.0, z]);
    positions.push([x + 1.0, y + 1.0, z]);
    positions.push([x, y + 1.0, z + 1.0]);
    
    positions.push([x + 1.0, y + 1.0, z]);
    positions.push([x + 1.0, y + 1.0, z + 1.0]);
    positions.push([x, y + 1.0, z + 1.0]);
    
    let normal = [0.0, 1.0, 0.0];
    for _ in 0..6 {
        normals.push(normal);
        uvs.push([0.0, 0.0]); // UV будут рассчитаны позже
        masks.push([mask, 0.0, 0.0, 1.0]); // R = маска, остальное 0
    }
}

/// Добавление правой грани (East) для Solid
fn add_right_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    masks: &mut Vec<[f32; 4]>,
    x: f32, y: f32, z: f32, mask: f32,
) {
    positions.push([x + 1.0, y, z]);
    positions.push([x + 1.0, y + 1.0, z]);
    positions.push([x + 1.0, y, z + 1.0]);
    
    positions.push([x + 1.0, y + 1.0, z]);
    positions.push([x + 1.0, y + 1.0, z + 1.0]);
    positions.push([x + 1.0, y, z + 1.0]);
    
    let normal = [1.0, 0.0, 0.0];
    for _ in 0..6 {
        normals.push(normal);
        uvs.push([0.0, 0.0]);
        masks.push([mask, 0.0, 0.0, 1.0]);
    }
}

/// Добавление левой грани (West) для Solid
fn add_left_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    masks: &mut Vec<[f32; 4]>,
    x: f32, y: f32, z: f32, mask: f32,
) {
    positions.push([x, y, z + 1.0]);
    positions.push([x, y + 1.0, z + 1.0]);
    positions.push([x, y, z]);
    
    positions.push([x, y + 1.0, z + 1.0]);
    positions.push([x, y + 1.0, z]);
    positions.push([x, y, z]);
    
    let normal = [-1.0, 0.0, 0.0];
    for _ in 0..6 {
        normals.push(normal);
        uvs.push([0.0, 0.0]);
        masks.push([mask, 0.0, 0.0, 1.0]);
    }
}

/// Добавление передней грани (South) для Solid
fn add_front_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    masks: &mut Vec<[f32; 4]>,
    x: f32, y: f32, z: f32, mask: f32,
) {
    positions.push([x, y, z + 1.0]);
    positions.push([x + 1.0, y, z + 1.0]);
    positions.push([x, y + 1.0, z + 1.0]);
    
    positions.push([x + 1.0, y, z + 1.0]);
    positions.push([x + 1.0, y + 1.0, z + 1.0]);
    positions.push([x, y + 1.0, z + 1.0]);
    
    let normal = [0.0, 0.0, 1.0];
    for _ in 0..6 {
        normals.push(normal);
        uvs.push([0.0, 0.0]);
        masks.push([mask, 0.0, 0.0, 1.0]);
    }
}

/// Добавление задней грани (North) для Solid
fn add_back_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    masks: &mut Vec<[f32; 4]>,
    x: f32, y: f32, z: f32, mask: f32,
) {
    positions.push([x + 1.0, y, z]);
    positions.push([x, y, z]);
    positions.push([x + 1.0, y + 1.0, z]);
    
    positions.push([x, y, z]);
    positions.push([x, y + 1.0, z]);
    positions.push([x + 1.0, y + 1.0, z]);
    
    let normal = [0.0, 0.0, -1.0];
    for _ in 0..6 {
        normals.push(normal);
        uvs.push([0.0, 0.0]);
        masks.push([mask, 0.0, 0.0, 1.0]);
    }
}

/// Добавление топ грани для Water
fn add_top_face_water(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    x: f32, y: f32, z: f32,
) {
    positions.push([x, y + 1.0, z]);
    positions.push([x + 1.0, y + 1.0, z]);
    positions.push([x, y + 1.0, z + 1.0]);
    
    positions.push([x + 1.0, y + 1.0, z]);
    positions.push([x + 1.0, y + 1.0, z + 1.0]);
    positions.push([x, y + 1.0, z + 1.0]);
    
    let normal = [0.0, 1.0, 0.0];
    for _ in 0..6 {
        normals.push(normal);
        uvs.push([0.0, 0.0]);
        colors.push([1.0, 1.0, 1.0, 0.5]); // Полупрозрачный белый
    }
}

/// Остальные функции для граней воды аналогично...
fn add_right_face_water(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    x: f32, y: f32, z: f32,
) {
    positions.push([x + 1.0, y, z]);
    positions.push([x + 1.0, y + 1.0, z]);
    positions.push([x + 1.0, y, z + 1.0]);
    
    positions.push([x + 1.0, y + 1.0, z]);
    positions.push([x + 1.0, y + 1.0, z + 1.0]);
    positions.push([x + 1.0, y, z + 1.0]);
    
    let normal = [1.0, 0.0, 0.0];
    for _ in 0..6 {
        normals.push(normal);
        uvs.push([0.0, 0.0]);
        colors.push([1.0, 1.0, 1.0, 0.5]);
    }
}

fn add_left_face_water(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    x: f32, y: f32, z: f32,
) {
    positions.push([x, y, z + 1.0]);
    positions.push([x, y + 1.0, z + 1.0]);
    positions.push([x, y, z]);
    
    positions.push([x, y + 1.0, z + 1.0]);
    positions.push([x, y + 1.0, z]);
    positions.push([x, y, z]);
    
    let normal = [-1.0, 0.0, 0.0];
    for _ in 0..6 {
        normals.push(normal);
        uvs.push([0.0, 0.0]);
        colors.push([1.0, 1.0, 1.0, 0.5]);
    }
}

fn add_front_face_water(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    x: f32, y: f32, z: f32,
) {
    positions.push([x, y, z + 1.0]);
    positions.push([x + 1.0, y, z + 1.0]);
    positions.push([x, y + 1.0, z + 1.0]);
    
    positions.push([x + 1.0, y, z + 1.0]);
    positions.push([x + 1.0, y + 1.0, z + 1.0]);
    positions.push([x, y + 1.0, z + 1.0]);
    
    let normal = [0.0, 0.0, 1.0];
    for _ in 0..6 {
        normals.push(normal);
        uvs.push([0.0, 0.0]);
        colors.push([1.0, 1.0, 1.0, 0.5]);
    }
}

fn add_back_face_water(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    x: f32, y: f32, z: f32,
) {
    positions.push([x + 1.0, y, z]);
    positions.push([x, y, z]);
    positions.push([x + 1.0, y + 1.0, z]);
    
    positions.push([x, y, z]);
    positions.push([x, y + 1.0, z]);
    positions.push([x + 1.0, y + 1.0, z]);
    
    let normal = [0.0, 0.0, -1.0];
    for _ in 0..6 {
        normals.push(normal);
        uvs.push([0.0, 0.0]);
        colors.push([1.0, 1.0, 1.0, 0.5]);
    }
}
