//! Утилиты для торической (циклической) математики.
//! 
//! Все функции используют модулярную арифметику для реализации
//! бесшовного мира с топологией тора.

/// Обернуть координату ячейки в пределах мира
#[inline]
pub const fn wrap_cell_coord(coord: i32, world_size: i32) -> i32 {
    // Быстрый модуль для положительных и отрицательных чисел
    ((coord % world_size) + world_size) % world_size
}

/// Обернуть координату чанка в пределах мира
#[inline]
pub const fn wrap_chunk_coord(coord: i32, world_size: i32) -> i32 {
    ((coord % world_size) + world_size) % world_size
}

/// Обернуть 3D координаты ячеек
#[inline]
pub const fn wrap_cell_coords(x: i32, y: i32, z: i32, world_size: i32) -> (i32, i32, i32) {
    (
        wrap_cell_coord(x, world_size),
        y,
        wrap_cell_coord(z, world_size),
    )
}

/// Обернуть 3D координаты чанков
#[inline]
pub const fn wrap_chunk_coords(x: i32, z: i32, world_size: i32) -> (i32, i32) {
    (
        wrap_chunk_coord(x, world_size),
        wrap_chunk_coord(z, world_size),
    )
}

/// Быстрый модуль через bitmask (работает только для степеней двойки)
#[inline]
pub const fn fast_mod(value: usize, mask: usize) -> usize {
    value & mask
}

/// Быстрое вычисление расстояния на торе (минимальное расстояние с учётом wrapping)
#[inline]
pub fn toroidal_distance(a: i32, b: i32, world_size: i32) -> i32 {
    let diff = (a - b).abs();
    diff.min(world_size - diff)
}

/// 3D расстояние на торе
#[inline]
pub fn toroidal_distance_3d(
    ax: i32, ay: i32, az: i32,
    bx: i32, by: i32, bz: i32,
    world_size: i32
) -> i32 {
    let dx = toroidal_distance(ax, bx, world_size);
    let dy = toroidal_distance(ay, by, world_size);
    let dz = toroidal_distance(az, bz, world_size);
    
    // Возвращаем манхэттенское расстояние (быстрее чем Euclidean)
    dx + dy + dz
}

/// Проверка, являются ли две позиции соседями на торе
#[inline]
pub fn are_neighbors_toroidal(
    ax: i32, ay: i32, az: i32,
    bx: i32, by: i32, bz: i32,
    world_size: i32
) -> bool {
    toroidal_distance_3d(ax, ay, az, bx, by, bz, world_size) <= 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_positive() {
        assert_eq!(wrap_cell_coord(5, 10), 5);
        assert_eq!(wrap_cell_coord(15, 10), 5);
        assert_eq!(wrap_cell_coord(100, 10), 0);
    }

    #[test]
    fn test_wrap_negative() {
        assert_eq!(wrap_cell_coord(-1, 10), 9);
        assert_eq!(wrap_cell_coord(-5, 10), 5);
        assert_eq!(wrap_cell_coord(-10, 10), 0);
        assert_eq!(wrap_cell_coord(-11, 10), 9);
    }

    #[test]
    fn test_wrap_zero() {
        assert_eq!(wrap_cell_coord(0, 10), 0);
    }

    #[test]
    fn test_fast_mod() {
        // mask = size - 1, где size - степень двойки
        let mask = 15; // size = 16
        assert_eq!(fast_mod(5, mask), 5);
        assert_eq!(fast_mod(16, mask), 0);
        assert_eq!(fast_mod(17, mask), 1);
        assert_eq!(fast_mod(31, mask), 15);
    }

    #[test]
    fn test_toroidal_distance() {
        // На мире размером 100
        assert_eq!(toroidal_distance(10, 20, 100), 10);
        assert_eq!(toroidal_distance(0, 99, 100), 1); // Соседи через границу
        assert_eq!(toroidal_distance(0, 50, 100), 50); // Максимальное расстояние
    }

    #[test]
    fn test_neighbors() {
        assert!(are_neighbors_toroidal(0, 0, 0, 1, 0, 0, 100));
        assert!(are_neighbors_toroidal(0, 0, 0, 0, 0, 0, 100)); // Та же позиция
        assert!(!are_neighbors_toroidal(0, 0, 0, 2, 0, 0, 100));
        
        // Соседи через границу
        assert!(are_neighbors_toroidal(0, 0, 0, 99, 0, 0, 100));
    }
}
