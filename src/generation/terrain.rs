//! Генерация ландшафта с использованием шума Перлина.
//! 
//! # Особенности:
//! - Детерминированная генерация по сиду
//! - 3D шум для плотности материалов
//! - Пороговые значения для определения типа блока

use noise::{NoiseFn, Seedable, SuperSimplex};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

use crate::core::{Cell, Chunk, ChunkPos, Material, CHUNK_SIZE, CHUNK_VOLUME};
use crate::utils::torus_math;

/// Параметры генерации мира
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenerationParams {
    /// Сид для генерации (должен быть одинаковым для воспроизводимости)
    pub seed: u64,
    
    /// Размер мира в чанках (должен быть степенью двойки)
    pub world_size_chunks: usize,
    
    /// Масштаб шума (чем больше, тем плавнее ландшафт)
    pub noise_scale: f64,
    
    /// Высота поверхности (в ячейках от низа мира)
    pub surface_height: i32,
    
    /// Разброс высоты (амплитуда неровностей)
    pub height_variation: i32,
    
    /// Частота деталей (для второго октавы шума)
    pub detail_frequency: f64,
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            seed: 42,
            world_size_chunks: 16,
            noise_scale: 0.05,
            surface_height: 64,
            height_variation: 32,
            detail_frequency: 2.0,
        }
    }
}

/// Генератор ландшафта
pub struct TerrainGenerator {
    params: GenerationParams,
    noise_main: SuperSimplex,
    noise_detail: SuperSimplex,
}

impl TerrainGenerator {
    /// Создать новый генератор с заданными параметрами
    pub fn new(params: GenerationParams) -> Self {
        let mut noise_main = SuperSimplex::new();
        let mut noise_detail = SuperSimplex::new();
        
        noise_main = noise_main.set_seed(params.seed as i32);
        noise_detail = noise_detail.set_seed((params.seed.wrapping_add(1)) as i32);
        
        Self {
            params,
            noise_main,
            noise_detail,
        }
    }

    /// Получить значение шума для координат
    #[inline]
    fn get_noise(&self, x: f64, y: f64, z: f64) -> f64 {
        let main = self.noise_main.get([x, y, z]);
        let detail = self.noise_detail.get([
            x * self.params.detail_frequency,
            y * self.params.detail_frequency,
            z * self.params.detail_frequency,
        ]);
        
        // Комбинировать основной шум с деталями
        main + detail * 0.3
    }

    /// Определить материал для ячейки на основе высоты и шума
    #[inline]
    fn get_material_at(&self, x: i32, y: i32, z: i32, density: f64) -> Material {
        // Нормализовать шум к диапазону [0, 1]
        let normalized = (density + 1.0) / 2.0;
        
        // Рассчитать целевую высоту поверхности в этой точке
        let surface_y = self.params.surface_height as f64 
            + self.params.height_variation as f64 * normalized;
        
        if y as f64 > surface_y + 5.0 {
            // Высоко над поверхностью - воздух
            Material::Air
        } else if y as f64 > surface_y {
            // Near surface - dirt or sand
            if normalized > 0.6 {
                Material::Stone
            } else {
                Material::Dirt
            }
        } else if y as f64 > surface_y - 10.0 {
            // Под поверхностью - камень или земля
            if normalized > 0.3 {
                Material::Stone
            } else {
                Material::Dirt
            }
        } else if y as f64 > surface_y - 20.0 {
            // Глубоко - камень
            Material::Stone
        } else {
            // Очень глубоко - магма
            Material::Magma
        }
    }

    /// Сгенерировать чанк на заданной позиции
    pub fn generate_chunk(&self, chunk_pos: ChunkPos) -> Chunk {
        let world_size_cells = self.params.world_size_chunks * CHUNK_SIZE;
        let mut cells = Vec::with_capacity(CHUNK_VOLUME);
        
        for local_z in 0..CHUNK_SIZE {
            for local_y in 0..CHUNK_SIZE {
                for local_x in 0..CHUNK_SIZE {
                    // Глобальные координаты
                    let global_x = chunk_pos.x as usize * CHUNK_SIZE + local_x;
                    let global_y = chunk_pos.y as usize * CHUNK_SIZE + local_y;
                    let global_z = chunk_pos.z as usize * CHUNK_SIZE + local_z;
                    
                    // Обернуть координаты для торического мира
                    let (wx, wy, wz) = torus_math::wrap_cell_coords(
                        global_x as i32,
                        global_y as i32,
                        global_z as i32,
                        world_size_cells as i32,
                    );
                    
                    // Нормализовать координаты для шума
                    let nx = wx as f64 * self.params.noise_scale;
                    let ny = wy as f64 * self.params.noise_scale;
                    let nz = wz as f64 * self.params.noise_scale;
                    
                    // Получить плотность из шума
                    let density = self.get_noise(nx, ny, nz);
                    
                    // Определить материал
                    let material = self.get_material_at(wx, wy, wz, density);
                    
                    cells.push(Cell::new(material));
                }
            }
        }
        
        Chunk::from_cells(chunk_pos, cells)
    }

    /// Заполнить менеджер чанков сгенерированными чанками
    pub fn generate_all_chunks<T>(&self, chunk_callback: &mut T)
    where
        T: FnMut(Chunk),
    {
        for cz in 0..self.params.world_size_chunks as i32 {
            for cy in 0..self.params.world_size_chunks as i32 {
                for cx in 0..self.params.world_size_chunks as i32 {
                    let chunk_pos = ChunkPos::new(cx, cy, cz);
                    let chunk = self.generate_chunk(chunk_pos);
                    chunk_callback(chunk);
                }
            }
        }
    }

    /// Получить параметры генерации
    pub fn params(&self) -> &GenerationParams {
        &self.params
    }

    /// Получить сид
    pub fn seed(&self) -> u64 {
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
    fn test_chunk_generation() {
        let params = GenerationParams {
            seed: 12345,
            world_size_chunks: 4,
            ..Default::default()
        };
        let gen = TerrainGenerator::new(params);
        
        let chunk = gen.generate_chunk(ChunkPos::new(0, 0, 0));
        assert_eq!(chunk.cells().len(), CHUNK_VOLUME);
        assert!(!chunk.is_empty()); // Должны быть какие-то блоки
    }

    #[test]
    fn test_deterministic_generation() {
        let params = GenerationParams::default();
        let gen1 = TerrainGenerator::new(params.clone());
        let gen2 = TerrainGenerator::new(params);
        
        let chunk1 = gen1.generate_chunk(ChunkPos::new(0, 0, 0));
        let chunk2 = gen2.generate_chunk(ChunkPos::new(0, 0, 0));
        
        // Одинаковый сид должен давать одинаковый результат
        assert_eq!(chunk1.cells().len(), chunk2.cells().len());
        
        // Проверить несколько ячеек
        for i in 0..100 {
            assert_eq!(chunk1.cells()[i].material, chunk2.cells()[i].material);
        }
    }

    #[test]
    fn test_different_seeds() {
        let gen1 = TerrainGenerator::new(GenerationParams { seed: 1, ..Default::default() });
        let gen2 = TerrainGenerator::new(GenerationParams { seed: 2, ..Default::default() });
        
        let chunk1 = gen1.generate_chunk(ChunkPos::new(0, 0, 0));
        let chunk2 = gen2.generate_chunk(ChunkPos::new(0, 0, 0));
        
        // Разные сиды должны давать разные результаты (хотя бы некоторые ячейки)
        let mut different = false;
        for i in 0..CHUNK_VOLUME {
            if chunk1.cells()[i].material != chunk2.cells()[i].material {
                different = true;
                break;
            }
        }
        assert!(different, "Разные сиды должны генерировать разные миры");
    }
}
