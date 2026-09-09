//! Chunk - контейнер для ячеек воксельного мира.
//! 
//! Архитектурные решения:
//! - Размер XZ: 16×16 (компактность для стриминга)
//! - Размер Y: 64 (фиксированная высота мира)
//! - Общий объём: 16384 ячейки на чанк
//! - Плоский Vec<Cell> с фиксированной ёмкостью
//! - Без аллокаций в горячих путях
//! - Поддержка торического wrapping при доступе

use crate::core::Cell;

/// Размер чанка по осям X и Z (степень двойки для оптимизации)
pub const CHUNK_SIZE_XZ: usize = 16;
/// Размер чанка по оси Y (фиксированная высота)
pub const CHUNK_SIZE_Y: usize = 64;
/// Общий размер чанка в ячейках: 16 × 16 × 64 = 16384
pub const CHUNK_VOLUME: usize = CHUNK_SIZE_XZ * CHUNK_SIZE_XZ * CHUNK_SIZE_Y;
/// Маска для быстрого вычисления модуля по XZ (CHUNK_SIZE_XZ - 1)
pub const CHUNK_MASK_XZ: usize = CHUNK_SIZE_XZ - 1;
/// Маска для быстрого вычисления модуля по Y (CHUNK_SIZE_Y - 1)
pub const CHUNK_MASK_Y: usize = CHUNK_SIZE_Y - 1;

/// Проверка, что размеры являются степенями двойки
const _: () = assert!(
    CHUNK_SIZE_XZ.is_power_of_two() && CHUNK_SIZE_Y.is_power_of_two(),
    "Размеры чанка должны быть степенями двойки"
);

/// Позиция чанка в мире (только XZ, Y всегда 0 для одного слоя)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// Преобразовать в ключ для HashMap (packed u64)
    #[inline]
    pub const fn to_key(&self) -> u64 {
        // Упаковать два i32 в u64: x(32 бита) | z(32 бита)
        ((self.x as u64 & 0xFFFFFFFF) << 32) | (self.z as u64 & 0xFFFFFFFF)
    }

    /// Создать из ключа
    #[inline]
    pub const fn from_key(key: u64) -> Self {
        let x = ((key >> 32) & 0xFFFFFFFF) as i32;
        let z = (key & 0xFFFFFFFF) as i32;
        Self { x, z }
    }
    
    /// Получить глобальные координаты начала чанка
    #[inline]
    pub const fn world_origin(&self) -> (isize, isize, isize) {
        (
            self.x as isize * CHUNK_SIZE_XZ as isize,
            0,
            self.z as isize * CHUNK_SIZE_XZ as isize,
        )
    }
}

impl std::fmt::Display for ChunkPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.z)
    }
}

/// Чанк - блок ячеек фиксированного размера.
/// 
/// # Память
/// Использует плоский Vec<Cell> с заранее выделенной памятью.
/// Ёмкость всегда равна CHUNK_VOLUME, реаллокации невозможны после создания.
/// 
/// # Layout
/// Порядок ячеек: row-major (x + z*SIZE_XZ + y*SIZE_XZ*SIZE_XZ)
/// Это обеспечивает лучшую локальность при обходе по XZ плоскости
#[repr(C)]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Chunk {
    /// Плоский массив ячеек
    cells: Vec<Cell>,
    
    /// Позиция чанка в мире (XZ)
    pub pos: ChunkPos,
    
    /// Флаг модификации (для оптимизации рендера и сериализации)
    pub is_dirty: bool,
    
    /// Статистика: количество активных (не-воздушных) ячеек
    pub active_cell_count: u32,
    
    /// Минимальная Y координата с активными ячейками (для оптимизации)
    pub min_y: u8,
    
    /// Максимальная Y координата с активными ячейками (для оптимизации)
    pub max_y: u8,
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("pos", &self.pos)
            .field("is_dirty", &self.is_dirty)
            .field("active_cell_count", &self.active_cell_count)
            .field("cells.len", &self.cells.len())
            .field("y_range", &(self.min_y..=self.max_y))
            .finish()
    }
}

impl Chunk {
    /// Создать новый пустой чанк на заданной позиции
    pub fn new(pos: ChunkPos) -> Self {
        // Предварительно выделить память под все ячейки
        let mut cells = Vec::with_capacity(CHUNK_VOLUME);
        // Заполнить воздухом
        cells.resize(CHUNK_VOLUME, Cell::AIR);
        
        Self {
            cells,
            pos,
            is_dirty: false,
            active_cell_count: 0,
            min_y: 0,
            max_y: 0,
        }
    }

    /// Создать чанк из готового массива ячеек (без проверки размера!)
    pub fn from_cells(pos: ChunkPos, cells: Vec<Cell>) -> Self {
        debug_assert_eq!(cells.len(), CHUNK_VOLUME);
        
        // Посчитать активные ячейки и диапазон Y
        let mut active_cell_count = 0u32;
        let mut min_y = CHUNK_SIZE_Y as u8;
        let mut max_y = 0u8;
        
        for (idx, cell) in cells.iter().enumerate() {
            if !cell.is_empty() {
                active_cell_count += 1;
                let y = (idx / (CHUNK_SIZE_XZ * CHUNK_SIZE_XZ)) as u8;
                if y < min_y { min_y = y; }
                if y > max_y { max_y = y; }
            }
        }
        
        Self {
            cells,
            pos,
            is_dirty: true,
            active_cell_count,
            min_y: if active_cell_count > 0 { min_y } else { 0 },
            max_y: if active_cell_count > 0 { max_y } else { 0 },
        }
    }

    /// Получить ссылку на ячейку по локальным координатам
    /// 
    /// # Panics
    /// Паникует в debug режиме, если координаты выходят за пределы
    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> &Cell {
        debug_assert!(x < CHUNK_SIZE_XZ && y < CHUNK_SIZE_Y && z < CHUNK_SIZE_XZ);
        &self.cells[self.index(x, y, z)]
    }

    /// Получить мутабельную ссылку на ячейку по локальным координатам
    #[inline]
    pub fn get_mut(&mut self, x: usize, y: usize, z: usize) -> &mut Cell {
        debug_assert!(x < CHUNK_SIZE_XZ && y < CHUNK_SIZE_Y && z < CHUNK_SIZE_XZ);
        self.is_dirty = true;
        self.update_y_range(y as u8);
        &mut self.cells[self.index(x, y, z)]
    }

    /// Установить ячейку по локальным координатам
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, cell: Cell) {
        debug_assert!(x < CHUNK_SIZE_XZ && y < CHUNK_SIZE_Y && z < CHUNK_SIZE_XZ);
        let idx = self.index(x, y, z);
        self.cells[idx] = cell;
        self.is_dirty = true;
        self.update_y_range(y as u8);
    }

    /// Получить индекс в плоском массиве по 3D координатам
    #[inline]
    const fn index(&self, x: usize, y: usize, z: usize) -> usize {
        // row-major order: x + z*SIZE_XZ + y*SIZE_XZ*SIZE_XZ
        x + z * CHUNK_SIZE_XZ + y * CHUNK_SIZE_XZ * CHUNK_SIZE_XZ
    }

    /// Получить индекс с торическим wrapping по XZ (циклическая топология)
    #[inline]
    const fn index_wrapped(&self, x: isize, y: isize, z: isize) -> usize {
        // Быстрый модуль через bitmask (работает только для степеней двойки)
        let x = (x & CHUNK_MASK_XZ as isize) as usize;
        let z = (z & CHUNK_MASK_XZ as isize) as usize;
        let y = (y & CHUNK_MASK_Y as isize) as usize;
        self.index(x, y, z)
    }

    /// Получить ячейку с торическим wrapping по XZ
    #[inline]
    pub fn get_wrapped(&self, x: isize, y: isize, z: isize) -> &Cell {
        &self.cells[self.index_wrapped(x, y, z)]
    }

    /// Получить мутабельную ссылку на ячейку с торическим wrapping
    #[inline]
    pub fn get_mut_wrapped(&mut self, x: isize, y: isize, z: isize) -> &mut Cell {
        self.is_dirty = true;
        &mut self.cells[self.index_wrapped(x, y, z)]
    }

    /// Обновить диапазон Y при изменении ячейки
    #[inline]
    fn update_y_range(&mut self, y: u8) {
        if !self.cells[self.index(0, y as usize, 0)].is_empty() {
            if y < self.min_y { self.min_y = y; }
            if y > self.max_y { self.max_y = y; }
        }
    }

    /// Итератор по всем ячейкам (для сериализации и генерации)
    #[inline]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Мутабельный срез всех ячеек
    #[inline]
    pub fn cells_mut(&mut self) -> &mut [Cell] {
        self.is_dirty = true;
        &mut self.cells
    }

    /// Проверка, полностью ли чанк пуст (только воздух)
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.active_cell_count == 0
    }

    /// Пересчитать счётчик активных ячеек и диапазон Y (медленно, использовать редко)
    pub fn recalculate_stats(&mut self) {
        self.active_cell_count = 0;
        self.min_y = CHUNK_SIZE_Y as u8;
        self.max_y = 0;
        
        for (idx, cell) in self.cells.iter().enumerate() {
            if !cell.is_empty() {
                self.active_cell_count += 1;
                let y = (idx / (CHUNK_SIZE_XZ * CHUNK_SIZE_XZ)) as u8;
                if y < self.min_y { self.min_y = y; }
                if y > self.max_y { self.max_y = y; }
            }
        }
        
        if self.active_cell_count == 0 {
            self.min_y = 0;
            self.max_y = 0;
        }
    }

    /// Заполнить чанк заданной ячейкой
    pub fn fill(&mut self, cell: Cell) {
        self.cells.fill(cell);
        self.active_cell_count = if cell.is_empty() { 0 } else { CHUNK_VOLUME as u32 };
        self.min_y = if cell.is_empty() { 0 } else { 0 };
        self.max_y = if cell.is_empty() { 0 } else { (CHUNK_SIZE_Y - 1) as u8 };
        self.is_dirty = true;
    }

    /// Очистить чанк (заполнить воздухом)
    pub fn clear(&mut self) {
        self.fill(Cell::AIR);
    }

    /// Получить размер чанка в байтах (для сериализации)
    pub const fn size_bytes() -> usize {
        CHUNK_VOLUME * std::mem::size_of::<Cell>()
    }
    
    /// Получить размеры чанка
    pub const fn dimensions() -> (usize, usize, usize) {
        (CHUNK_SIZE_XZ, CHUNK_SIZE_Y, CHUNK_SIZE_XZ)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Material;

    #[test]
    fn test_chunk_volume() {
        assert_eq!(CHUNK_VOLUME, 16 * 16 * 64);
        assert_eq!(CHUNK_VOLUME, 16384);
    }

    #[test]
    fn test_chunk_creation() {
        let chunk = Chunk::new(ChunkPos::new(0, 0));
        assert_eq!(chunk.cells.len(), CHUNK_VOLUME);
        assert_eq!(chunk.active_cell_count, 0);
    }

    #[test]
    fn test_chunk_get_set() {
        let mut chunk = Chunk::new(ChunkPos::new(0, 0));
        
        chunk.set(0, 0, 0, Cell::new(Material::Stone));
        assert!(chunk.get(0, 0, 0).is_solid());
        
        assert!(chunk.is_dirty);
    }

    #[test]
    fn test_chunk_wrapping_xz() {
        let mut chunk = Chunk::new(ChunkPos::new(0, 0));
        
        // Установить ячейку на границе XZ
        chunk.set(CHUNK_SIZE_XZ - 1, 32, CHUNK_SIZE_XZ - 1, Cell::new(Material::Water));
        
        // Проверить wrapping по X
        let wrapped_x = chunk.get_wrapped(CHUNK_SIZE_XZ as isize, 32, 0);
        assert!(!wrapped_x.is_empty());
        
        // Точная проверка wrapping
        chunk.set(0, 32, 0, Cell::new(Material::Stone));
        let at_boundary = chunk.get_wrapped(-1, 32, 0);
        assert_eq!(at_boundary.get_material(), Material::Water); // x=-1 -> x=SIZE-1
    }

    #[test]
    fn test_chunk_pos_hash() {
        let pos = ChunkPos::new(1, -2);
        let key = pos.to_key();
        let restored = ChunkPos::from_key(key);
        assert_eq!(pos, restored);
    }

    #[test]
    fn test_chunk_index_order() {
        let chunk = Chunk::new(ChunkPos::new(0, 0));
        
        // Проверить правильный порядок индексов
        let idx0 = chunk.index(0, 0, 0);
        let idx1 = chunk.index(1, 0, 0);
        let idx2 = chunk.index(0, 0, 1);
        let idx3 = chunk.index(0, 1, 0);
        
        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
        assert_eq!(idx2, CHUNK_SIZE_XZ);
        assert_eq!(idx3, CHUNK_SIZE_XZ * CHUNK_SIZE_XZ);
    }
    
    #[test]
    fn test_y_range() {
        let mut chunk = Chunk::new(ChunkPos::new(0, 0));
        assert_eq!(chunk.min_y, 0);
        assert_eq!(chunk.max_y, 0);
        
        // Установить ячейку на высоте 50
        chunk.set(8, 50, 8, Cell::new(Material::Stone));
        chunk.recalculate_stats();
        
        assert_eq!(chunk.min_y, 50);
        assert_eq!(chunk.max_y, 50);
    }
}
