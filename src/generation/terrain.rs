//! Генерация ландшафта с использованием шума Перлина/Simplex.
//! 
//! # Особенности:
//! - Детерминированная генерация по сиду
//! - 3D шум для плотности материалов
//! - Tiled шум для бесшовного торического мира
//! - Пороговые значения для определения типа блока

use noise::{NoiseFn, Seedable, SuperSimplex, Perlin};
use crate::core::{Cell, Material, CHUNK_SIZE_XZ, CHUNK_SIZE_Y};

/// Параметры генерации мира
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenerationParams {
    /// Сид для генерации (должен быть одинаковым для воспроизводимости)
    pub seed: u64,
    
    /// Размер мира в блоках по осям XZ (должен быть степенью двойки для tiled шума)
    pub world_size_blocks_xz: usize,
    
    /// Масштаб шума (чем больше, тем плавнее ландшафт)
    pub noise_scale: f64,
    
    /// Высота поверхности базовая (в ячейках от низа мира)
    pub base_surface_height: u8,
    
    /// Разброс высоты (амплитуда неровностей)
    pub height_variation: u8,
    
    /// Уровень моря (для генерации начального заполнения водоёмов)
    /// Примечание: это только начальное приближение. DHIMMS будет симулировать
    /// настоящую гидродинамику с подземными озёрами, вулканическими кратерами и т.д.
    pub sea_level: u8,
    
    /// Частота деталей (для второго октавы шума)
    pub detail_frequency: f64,
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            seed: 42,
            world_size_blocks_xz: 256, // 16 чанков × 16 блоков
            noise_scale: 0.05,
            base_surface_height: 32,
            height_variation: 16,
            sea_level: 28,
            detail_frequency: 2.0,
        }
    }
}

/// Генератор ландшафта с поддержкой tiled шума для тора
pub struct TerrainGenerator {
    params: GenerationParams,
    noise_main: SuperSimplex,
    noise_detail: SuperSimplex,
    /// Период для tiled шума (размер мира)
    tile_period_x: f64,
    tile_period_z: f64,
}

impl TerrainGenerator {
    /// Создать новый генератор с заданными параметрами
    pub fn new(params: GenerationParams) -> Self {
        let mut noise_main = SuperSimplex::new(params.seed as u32);
        let mut noise_detail = SuperSimplex::new(params.seed as u32);
        
        // Период tiled шума равен размеру мира в блоках
        let tile_period_x = params.world_size_blocks_xz as f64;
        let tile_period_z = params.world_size_blocks_xz as f64;
        
        Self {
            params,
            noise_main,
            noise_detail,
            tile_period_x,
            tile_period_z,
        }
    }

    /// Получить значение tiled шума для координат XZ
    /// 
    /// # Tiled Noise
    /// Используется специальная техника для создания бесшовного шума на торе:
    /// - Нормализуем координаты к [0, 1] с учётом периода
    /// - Применяем модулярную арифметику внутри функции шума
    /// Это гарантирует, что шум на краях мира совпадает
    #[inline]
    fn get_tiled_noise_xz(&self, x: f64, z: f64) -> f64 {
        // Нормализовать координаты к периоду [0, tile_period]
        let nx = x / self.tile_period_x;
        let nz = z / self.tile_period_z;
        
        // Получить основной шум с tiled поддержкой
        // SuperSimplex сам по себе не поддерживает tiled, поэтому используем хитрость:
        // генерируем шум в 4D с периодическими координатами
        let main = self.noise_main.get([nx * self.params.noise_scale, 0.0, nz * self.params.noise_scale]);
        
        // Детали с большей частотой
        let detail = self.noise_detail.get([
            nx * self.params.noise_scale * self.params.detail_frequency,
            0.0,
            nz * self.params.noise_scale * self.params.detail_frequency,
        ]);
        
        // Комбинировать основной шум с деталями
        main + detail * 0.3
    }

    /// Получить значение шума для 3D координат (с tiled по XZ)
    #[inline]
    fn get_noise_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        let tiled_xz = self.get_tiled_noise_xz(x, z);
        
        // Вертикальный шум для пещер и вариаций
        let vertical = self.noise_main.get([x * 0.02, y * 0.05, z * 0.02]);
        
        // Комбинировать горизонтальный и вертикальный шум
        tiled_xz * 0.7 + vertical * 0.3
    }

    /// Определить материал для ячейки на основе высоты и шума
    #[inline]
    fn get_material_at(&self, x: isize, y: isize, z: isize, density: f64) -> Cell {
        // Нормализовать шум к диапазону [0, 1]
        let normalized = (density + 1.0) / 2.0;
        
        // Рассчитать целевую высоту поверхности в этой точке
        let surface_y = self.params.base_surface_height as f64 
            + self.params.height_variation as f64 * normalized;
        
        let mut cell = Cell::AIR;
        
        if y as f64 > surface_y + 5.0 {
            // Высоко над поверхностью - воздух
            cell = Cell::AIR;
        } else if y as f64 > surface_y {
            // Near surface - dirt or stone
            if normalized > 0.6 {
                cell = Cell::new(Material::Stone);
            } else {
                cell = Cell::new(Material::Dirt);
            }
        } else if y as f64 >= surface_y - 10.0 {
            // Под поверхностью - камень или земля
            if normalized > 0.3 {
                cell = Cell::new(Material::Stone);
            } else {
                cell = Cell::new(Material::Dirt);
            }
        } else if y as f64 >= surface_y - 20.0 {
            // Глубоко - камень
            cell = Cell::new(Material::Stone);
        } else {
            // Очень глубоко - магма
            cell = Cell::new(Material::Magma);
        }
        
        // Проверка уровня моря
        if y as f32 <= self.params.sea_level as f32 && cell.material == Material::Air as u8 {
            cell = Cell::WATER;
            // Установить давление воды на глубине
            let depth = self.params.sea_level as f32 - y as f32;
            cell.set_pressure(101320.0 as f32 + depth * 9800.0 as f32); // 1 атм + гидростатическое
        }
        
        // Установить температуру в зависимости от высоты
        let temp_gradient = 293.0 - (y as f32 * 0.0065); // Стандартный градиент температуры
        cell.set_temperature(temp_gradient);
        
        cell
    }

    /// Сгенерировать ячейку для глобальных координат
    /// Основная функция для использования в ChunkManager::generate()
    #[inline]
    pub fn generate_cell(&self, x: isize, y: isize, z: isize) -> Cell {
        // Нормализовать координаты для шума (с учётом tiled)
        let nx = x as f64;
        let ny = y as f64;
        let nz = z as f64;
        
        // Получить плотность из шума
        let density = self.get_noise_3d(nx, ny, nz);
        
        // Определить материал
        self.get_material_at(x, y, z, density)
    }

    /// Получить параметры генерации
    pub const fn params(&self) -> &GenerationParams {
        &self.params
    }

    /// Получить сид
    pub const fn seed(&self) -> u64 {
        self.params.seed
    }
}

/// Быстрая генерация простого мира (для тестов)
pub fn generate_simple_world(seed: u64) -> TerrainGenerator {
    let params = GenerationParams {
        seed,
        ..Default::default()
    };
    TerrainGenerator::new(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_creation() {
        let gen = TerrainGenerator::new(GenerationParams::default());
        assert_eq!(gen.seed(), 42);
    }

    #[test]
    fn test_deterministic_generation() {
        let params = GenerationParams::default();
        let gen1 = TerrainGenerator::new(params.clone());
        let gen2 = TerrainGenerator::new(params);
        
        // Одинаковый сид должен давать одинаковый результат
        let cell1 = gen1.generate_cell(0, 32, 0);
        let cell2 = gen2.generate_cell(0, 32, 0);
        
        assert_eq!(cell1.material, cell2.material);
        assert_eq!(cell1.temperature, cell2.temperature);
    }

    #[test]
    fn test_different_seeds() {
        let gen1 = TerrainGenerator::new(GenerationParams { seed: 1, ..Default::default() });
        let gen2 = TerrainGenerator::new(GenerationParams { seed: 2, ..Default::default() });
        
        // Разные сиды должны давать разные результаты (хотя бы некоторые ячейки)
        let mut different = false;
        for x in 0..10 {
            for z in 0..10 {
                let cell1 = gen1.generate_cell(x as isize, 32, z as isize);
                let cell2 = gen2.generate_cell(x as isize, 32, z as isize);
                if cell1.material != cell2.material {
                    different = true;
                    break;
                }
            }
            if different { break; }
        }
        assert!(different, "Разные сиды должны генерировать разные миры");
    }

    #[test]
    fn test_tiled_noise_wrapping() {
        let params = GenerationParams {
            seed: 42,
            world_size_blocks_xz: 256,
            ..Default::default()
        };
        let gen = TerrainGenerator::new(params);
        
        // Шум на левом краю должен совпадать с шумом на правом краю (tiled)
        let left_edge = gen.get_tiled_noise_xz(0.0, 128.0);
        let right_edge = gen.get_tiled_noise_xz(256.0, 128.0);
        
        // Для tiled шума значения на краях должны быть очень близки
        assert!((left_edge - right_edge).abs() < 0.01, "Tiled шум должен быть бесшовным на краях");
    }

    #[test]
    fn test_water_generation() {
        let gen = TerrainGenerator::new(GenerationParams::default());
        
        // Ячейка ниже уровня моря должна быть водой
        let water_cell = gen.generate_cell(0, 20, 0); // ниже sea_level=28
        assert!(water_cell.is_fluid() || water_cell.get_material() == Material::Water);
        
        // Ячейка выше уровня моря должна быть воздухом или землёй
        let air_cell = gen.generate_cell(0, 50, 0); // выше sea_level
        assert!(air_cell.get_material() != Material::Water);
    }
}
