//! Chunk - контейнер для ячеек воксельного мира.
//! 
//! Оптимизированная структура для хранения блока ячеек:
//! - Плоский Vec<Cell> с фиксированной ёмкостью
//! - Без аллокаций в горячих путях
//! - Поддержка торического wrapping при доступе

use crate::core::{Cell, CELL_SIZE};

/// Размер чанка по каждой оси (должен быть степенью двойки для оптимизации)
pub const CHUNK_SIZE: usize = 32;
/// Общий размер чанка в ячейках
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
/// Маска для быстрого вычисления модуля (CHUNK_SIZE - 1)
pub const CHUNK_MASK: usize = CHUNK_SIZE - 1;

/// Проверка, что CHUNK_SIZE является степенью двойки
const _: () = assert!(
    CHUNK_SIZE.is_power_of_two(),
    "CHUNK_SIZE должен быть степенью двойки"
);

/// Позиция чанка в мире
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Преобразовать в ключ для HashMap (packed u64)
    #[inline]
    pub const fn to_key(&self) -> u64 {
        // Упаковать три i32 в u64: x(20 бит) | y(20 бит) | z(20 бит) + sign bits
        // Используем простое хеширование для скорости
        ((self.x as u64 & 0xFFFFF) << 42)
            | ((self.y as u64 & 0xFFFFF) << 21)
            | (self.z as u64 & 0xFFFFF)
    }

    /// Создать из ключа
    #[inline]
    pub const fn from_key(key: u64) -> Self {
        let x = ((key >> 42) & 0xFFFFF) as i32;
        let y = ((key >> 21) & 0xFFFFF) as i32;
        let z = (key & 0xFFFFF) as i32;
        
        // Расширить знаковый бит
        let x = if x & 0x80000 != 0 { x | !0xFFFFF } else { x };
        let y = if y & 0x80000 != 0 { y | !0xFFFFF } else { y };
        let z = if z & 0x80000 != 0 { z | !0xFFFFF } else { z };
        
        Self { x, y, z }
    }
}

impl std::fmt::Display for ChunkPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

/// Чанк - блок ячеек фиксированного размера.
/// 
/// # Память
/// Использует плоский Vec<Cell> с заранее выделенной памятью.
/// Ёмкость всегда равна CHUNK_VOLUME, реаллокации невозможны после создания.
#[repr(C)]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Chunk {
    /// Плоский массив ячеек (row-major order: x + y*SIZE + z*SIZE*SIZE)
    cells: Vec<Cell>,
    
    /// Позиция чанка в мире
    pub pos: ChunkPos,
    
    /// Флаг модификации (для оптимизации рендера и сериализации)
    pub is_dirty: bool,
    
    /// Статистика: количество активных (не-воздушных) ячеек
    pub active_cell_count: u32,
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("pos", &self.pos)
            .field("is_dirty", &self.is_dirty)
            .field("active_cell_count", &self.active_cell_count)
            .field("cells.len", &self.cells.len())
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
        }
    }

    /// Создать чанк из готового массива ячеек (без проверки размера!)
    pub fn from_cells(pos: ChunkPos, cells: Vec<Cell>) -> Self {
        debug_assert_eq!(cells.len(), CHUNK_VOLUME);
        
        // Посчитать активные ячейки
        let active_cell_count = cells.iter().filter(|c| !c.is_empty()).count() as u32;
        
        Self {
            cells,
            pos,
            is_dirty: true,
            active_cell_count,
        }
    }

    /// Получить ссылку на ячейку по локальным координатам
    /// 
    /// # Panics
    /// Паникует в debug режиме, если координаты выходят за пределы [0, CHUNK_SIZE)
    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> &Cell {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        &self.cells[self.index(x, y, z)]
    }

    /// Получить мутабельную ссылку на ячейку по локальным координатам
    /// 
    /// # Panics
    /// Паникует в debug режиме, если координаты выходят за пределы [0, CHUNK_SIZE)
    #[inline]
    pub fn get_mut(&mut self, x: usize, y: usize, z: usize) -> &mut Cell {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        self.is_dirty = true;
        &mut self.cells[self.index(x, y, z)]
    }

    /// Установить ячейку по локальным координатам
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, cell: Cell) {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        let idx = self.index(x, y, z);
        self.cells[idx] = cell;
        self.is_dirty = true;
    }

    /// Получить индекс в плоском массиве по 3D координатам
    #[inline]
    const fn index(&self, x: usize, y: usize, z: usize) -> usize {
        // row-major order: x + y*SIZE + z*SIZE*SIZE
        x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE
    }

    /// Получить индекс с торическим wrapping (циклическая топология)
    #[inline]
    const fn index_wrapped(&self, x: isize, y: isize, z: isize) -> usize {
        // Быстрый модуль через bitmask (работает только для степеней двойки)
        let x = (x & CHUNK_MASK as isize) as usize;
        let y = (y & CHUNK_MASK as isize) as usize;
        let z = (z & CHUNK_MASK as isize) as usize;
        self.index(x, y, z)
    }

    /// Получить ячейку с торическим wrapping
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

    /// Пересчитать счётчик активных ячеек (медленно, использовать редко)
    pub fn recalculate_active_count(&mut self) {
        self.active_cell_count = self.cells.iter().filter(|c| !c.is_empty()).count() as u32;
    }

    /// Заполнить чанк заданной ячейкой
    pub fn fill(&mut self, cell: Cell) {
        self.cells.fill(cell);
        self.active_cell_count = if cell.is_empty() { 0 } else { CHUNK_VOLUME as u32 };
        self.is_dirty = true;
    }

    /// Очистить чанк (заполнить воздухом)
    pub fn clear(&mut self) {
        self.fill(Cell::AIR);
    }

    /// Получить размер чанка в байтах (для сериализации)
    pub const fn size_bytes() -> usize {
        CHUNK_VOLUME * CELL_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Material;

    #[test]
    fn test_chunk_creation() {
        let chunk = Chunk::new(ChunkPos::new(0, 0, 0));
        assert_eq!(chunk.cells.len(), CHUNK_VOLUME);
        assert_eq!(chunk.active_cell_count, 0);
    }

    #[test]
    fn test_chunk_get_set() {
        let mut chunk = Chunk::new(ChunkPos::new(0, 0, 0));
        
        chunk.set(0, 0, 0, Cell::new(Material::Stone));
        assert!(chunk.get(0, 0, 0).is_solid());
        
        assert!(chunk.is_dirty);
    }

    #[test]
    fn test_chunk_wrapping() {
        let mut chunk = Chunk::new(ChunkPos::new(0, 0, 0));
        
        // Установить ячейку на границе
        chunk.set(CHUNK_SIZE - 1, CHUNK_SIZE - 1, CHUNK_SIZE - 1, Cell::new(Material::Water));
        
        // Проверить wrapping: выход за границу должен вернуть ячейку с противоположной стороны
        let wrapped = chunk.get_wrapped(CHUNK_SIZE as isize, 0, 0);
        assert!(!wrapped.is_empty()); // Должна вернуть ячейку с x=0
        
        // Точная проверка wrapping
        chunk.set(0, 0, 0, Cell::new(Material::Stone));
        let at_boundary = chunk.get_wrapped(-1, 0, 0);
        assert_eq!(at_boundary.get_material(), Material::Water); // x=-1 -> x=SIZE-1
    }

    #[test]
    fn test_chunk_pos_hash() {
        let pos = ChunkPos::new(1, -2, 3);
        let key = pos.to_key();
        let restored = ChunkPos::from_key(key);
        assert_eq!(pos, restored);
    }

    #[test]
    fn test_chunk_index_order() {
        let chunk = Chunk::new(ChunkPos::new(0, 0, 0));
        
        // Проверить правильный порядок индексов
        let idx0 = chunk.index(0, 0, 0);
        let idx1 = chunk.index(1, 0, 0);
        let idx2 = chunk.index(0, 1, 0);
        let idx3 = chunk.index(0, 0, 1);
        
        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
        assert_eq!(idx2, CHUNK_SIZE);
        assert_eq!(idx3, CHUNK_SIZE * CHUNK_SIZE);
    }
}
