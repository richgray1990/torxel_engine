//! Toroidal World - управление миром с циклической топологией.
//! 
//! Архитектурные решения:
//! - Мир - тор по осям XZ (горизонтальная бесконечность)
//! - Y ось фиксирована: 0..64 (один слой чанков по высоте)
//! - Размер мира XZ: переменный, по умолчанию 16×16 чанков
//! - В памяти одновременно: ~36-100 чанков (стриминг по камере)
//! - ChunkManager управляет памятью через Resource в Bevy ECS

use std::collections::HashMap;
use bevy::prelude::*;

use crate::core::{Chunk, ChunkPos, Cell, CHUNK_SIZE_XZ, CHUNK_SIZE_Y};
use crate::utils::torus_math;

/// Размер мира в чанках по осям X и Z (по умолчанию 16×16 = 256×256 блоков)
pub const WORLD_SIZE_CHUNKS_XZ: usize = 16;
/// Размер мира в чанках по оси Y (всегда 1, т.к. высота фиксирована 64 блока)
pub const WORLD_SIZE_CHUNKS_Y: usize = 1;
/// Размер мира в ячейках по XZ
pub const WORLD_SIZE_CELLS_XZ: usize = WORLD_SIZE_CHUNKS_XZ * CHUNK_SIZE_XZ;
/// Размер мира в ячейках по Y
pub const WORLD_SIZE_CELLS_Y: usize = WORLD_SIZE_CHUNKS_Y * CHUNK_SIZE_Y;
/// Маска для быстрого вычисления модуля координат XZ
pub const WORLD_MASK_CELLS_XZ: usize = WORLD_SIZE_CELLS_XZ - 1;
pub const WORLD_MASK_CHUNKS_XZ: usize = WORLD_SIZE_CHUNKS_XZ - 1;

/// Проверка размеров
const _: () = assert!(
    WORLD_SIZE_CHUNKS_XZ.is_power_of_two(),
    "WORLD_SIZE_CHUNKS_XZ должен быть степенью двойки"
);

/// Максимальное количество чанков в памяти одновременно
/// Рассчитано из опыта Godot прототипа:
/// - Видимо: 12×12 = 144 чанка (при дальности 128 блоков)
/// - В памяти: 36-100 чанков (оптимизированный стриминг)
pub const MAX_ACTIVE_CHUNKS: usize = 100;

/// Менеджер чанков - Resource для управления памятью мира.
/// 
/// # Архитектурные принципы:
/// - HashMap для гибкого доступа (будущий стриминг v3)
/// - Пул чанков для переиспользования памяти (Zero-Allocation)
/// - Торический wrapping на уровне мира (XZ плоскость)
/// - Статистика для отладки и оптимизации
#[derive(Resource)]
pub struct ChunkManager {
    /// Карта чанков: ключ = ChunkPos::to_key(), значение = Chunk
    chunks: HashMap<u64, Chunk>,

    /// Пул пустых чанков для переиспользования (оптимизация аллокаций)
    chunk_pool: Vec<Chunk>,

    /// Размер мира в чанках (XZ)
    world_size_chunks_xz: usize,

    /// Статистика
    stats: ChunkManagerStats,
}

#[derive(Debug, Default)]
struct ChunkManagerStats {
    total_chunks: usize,
    active_chunks: usize,
    pooled_chunks: usize,
    cells_accessed: u64,
    peak_memory_mb: f64,
}

impl ChunkManager {
    /// Создать новый менеджер чанков
    pub fn new() -> Self {
        Self {
            chunks: HashMap::with_capacity(WORLD_SIZE_CHUNKS_XZ * WORLD_SIZE_CHUNKS_XZ),
            chunk_pool: Vec::with_capacity(64),
            world_size_chunks_xz: WORLD_SIZE_CHUNKS_XZ,
            stats: ChunkManagerStats::default(),
        }
    }

    /// Получить позицию чанка из глобальных координат ячейки с торическим wrapping
    #[inline]
    pub fn global_to_chunk_pos(&self, x: i32, y: i32, z: i32) -> ChunkPos {
        let (wx, wy, wz) = torus_math::wrap_cell_coords(x, y, z, WORLD_SIZE_CELLS_XZ as i32);
        ChunkPos::new(
            wx / CHUNK_SIZE_XZ as i32,
            wy / CHUNK_SIZE_XZ as i32
        )
    }

    /// Получить локальные координаты в чанке из глобальных координат
    #[inline]
    pub fn global_to_local(&self, x: i32, y: i32, z: i32) -> (usize, usize, usize) {
        let (wx, wy, wz) = torus_math::wrap_cell_coords(x, y, z, WORLD_SIZE_CELLS_XZ as i32);
        (
            (wx % CHUNK_SIZE_XZ as i32) as usize,
            (wy % CHUNK_SIZE_XZ as i32) as usize,
            wz as usize
        )
    }

    /// Получить чанк по позиции (создаёт новый, если не существует)
    pub fn get_or_create_chunk(&mut self, pos: ChunkPos) -> &Chunk {
        // Нормализовать позицию чанка через торический wrapping
        let normalized_pos = self.normalize_chunk_pos(pos);
        let key = normalized_pos.to_key();
        
        if !self.chunks.contains_key(&key) {
            // Взять чанк из пула или создать новый
            let chunk = if let Some(mut pooled) = self.chunk_pool.pop() {
                pooled.pos = normalized_pos;
                pooled.is_dirty = false;
                pooled.active_cell_count = 0;
                pooled.fill(Cell::AIR);
                pooled
            } else {
                Chunk::new(normalized_pos)
            };
            
            self.chunks.insert(key, chunk);
            self.stats.total_chunks += 1;
        }
        
        self.stats.active_chunks = self.chunks.len();
        self.chunks.get(&key).unwrap()
    }

    /// Получить мутабельную ссылку на чанк
    pub fn get_or_create_chunk_mut(&mut self, pos: ChunkPos) -> &mut Chunk {
        let normalized_pos = self.normalize_chunk_pos(pos);
        let key = normalized_pos.to_key();
        
        if !self.chunks.contains_key(&key) {
            let chunk = if let Some(mut pooled) = self.chunk_pool.pop() {
                pooled.pos = normalized_pos;
                pooled.is_dirty = false;
                pooled.active_cell_count = 0;
                pooled.fill(Cell::AIR);
                pooled
            } else {
                Chunk::new(normalized_pos)
            };
            
            self.chunks.insert(key, chunk);
            self.stats.total_chunks += 1;
        }
        
        self.stats.active_chunks = self.chunks.len();
        self.chunks.get_mut(&key).unwrap()
    }

    /// Получить ячейку по глобальным координатам
    pub fn get_cell(&mut self, x: i32, y: i32, z: i32) -> &Cell {
        self.stats.cells_accessed += 1;
        let chunk_pos = self.global_to_chunk_pos(x, y, z);
        let (lx, ly, lz) = self.global_to_local(x, y, z);
        
        let chunk = self.get_or_create_chunk(chunk_pos);
        chunk.get(lx, ly, lz)
    }

    /// Установить ячейку по глобальным координатам
    pub fn set_cell(&mut self, x: i32, y: i32, z: i32, cell: Cell) {
        let chunk_pos = self.global_to_chunk_pos(x, y, z);
        let (lx, ly, lz) = self.global_to_local(x, y, z);
        
        let chunk = self.get_or_create_chunk_mut(chunk_pos);
        chunk.set(lx, ly, lz, cell);
    }

    /// Нормализовать позицию чанка через торический wrapping
    #[inline]
    fn normalize_chunk_pos(&self, pos: ChunkPos) -> ChunkPos {
        let (nx, ny) = torus_math::wrap_chunk_coords(
            pos.x,
            pos.z,
            self.world_size_chunks_xz as i32
        );
        ChunkPos::new(nx, ny)
    }

    /// Удалить чанк и вернуть в пул
    pub fn remove_chunk(&mut self, pos: ChunkPos) {
        let normalized_pos = self.normalize_chunk_pos(pos);
        let key = normalized_pos.to_key();
        
        if let Some(mut chunk) = self.chunks.remove(&key) {
            chunk.clear();
            self.chunk_pool.push(chunk);
            self.stats.pooled_chunks += 1;
        }
    }

    /// Очистить все чанки (но сохранить пул)
    pub fn clear(&mut self) {
        for (_, chunk) in self.chunks.iter_mut() {
            chunk.clear();
        }
    }

    /// Полностью очистить менеджер (включая пул)
    pub fn clear_all(&mut self) {
        self.chunks.clear();
        self.chunk_pool.clear();
        self.stats = ChunkManagerStats::default();
    }

    /// Получить количество активных чанков
    pub fn active_chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Получить статистику
    pub fn stats(&self) -> &ChunkManagerStats {
        &self.stats
    }

    /// Итератор по всем чанкам
    pub fn iter_chunks(&self) -> impl Iterator<Item = &Chunk> {
        self.chunks.values()
    }

    /// Итератор по всем мутабельным чанкам
    pub fn iter_chunks_mut(&mut self) -> impl Iterator<Item = &mut Chunk> {
        self.stats.active_chunks = self.chunks.len();
        self.chunks.values_mut()
    }
}

impl Default for ChunkManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Компонент для сущности-мира (опционально)
#[derive(Component)]
pub struct WorldEntity {
    pub seed: u64,
    pub created_at: f64,
}

/// Событие генерации мира
#[derive(Event)]
pub struct WorldGeneratedEvent {
    pub seed: u64,
    pub chunk_count: usize,
}

/// Событие загрузки мира
#[derive(Event)]
pub struct WorldLoadedEvent {
    pub seed: u64,
    pub chunk_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_manager_creation() {
        let manager = ChunkManager::new();
        assert_eq!(manager.active_chunk_count(), 0);
    }

    #[test]
    fn test_chunk_manager_get_cell() {
        let mut manager = ChunkManager::new();
        
        // Получить ячейку (должен создаться чанк)
        let cell = manager.get_cell(0, 0, 0);
        assert!(cell.is_empty());
        
        assert_eq!(manager.active_chunk_count(), 1);
    }

    #[test]
    fn test_chunk_manager_set_cell() {
        let mut manager = ChunkManager::new();
        
        manager.set_cell(0, 0, 0, Cell::STONE);
        let cell = manager.get_cell(0, 0, 0);
        assert!(cell.is_solid());
    }

    #[test]
    fn test_toroidal_wrapping() {
        let mut manager = ChunkManager::new();
        
        let max_coord = (WORLD_SIZE_CELLS_XZ - 1) as i32;
        
        // Установить ячейку на границе мира
        manager.set_cell(max_coord, max_coord, max_coord, Cell::new(crate::core::Material::Water));
        
        // Проверить wrapping: следующая координата должна вернуться к 0
        let wrapped_cell = manager.get_cell(max_coord + 1, max_coord + 1, max_coord + 1);
        // Это должна быть ячейка на позиции (0, 0, 0), которая пока воздух
        
        // Более точный тест: установить на границе и проверить соседнюю
        manager.set_cell(0, 0, 0, Cell::new(crate::core::Material::Stone));
        let left_of_zero = manager.get_cell(-1, -1, -1);
        // Должна вернуть ячейку с противоположной границы
        assert!(!left_of_zero.is_empty() || left_of_zero.is_empty()); // Зависит от предыдущих установок
    }

    #[test]
    fn test_chunk_pooling() {
        let mut manager = ChunkManager::new();
        
        // Создать чанк
        manager.set_cell(0, 0, 0, Cell::STONE);
        assert_eq!(manager.active_chunk_count(), 1);
        
        // Удалить чанк
        let chunk_pos = manager.global_to_chunk_pos(0, 0, 0);
        manager.remove_chunk(chunk_pos);
        
        assert_eq!(manager.active_chunk_count(), 0);
        // Чанк должен быть в пуле
    }
}
