//! Cell - базовая единица воксельного мира.
//! 
//! Оптимизированная структура для хранения состояния вокселя:
//! - #[repr(C)] для предсказуемого расположения в памяти
//! - Материал как u8 (до 256 типов)
//! - Битовые флаги для быстрого доступа к свойствам
//! - Квантованные температура и давление (i16)
//! - Скорость как packed i8 вектор

use std::mem;

/// Размер одного материала в байтах
pub const MATERIAL_SIZE: usize = 1;
/// Размер температуры/давления в байтах
pub const TEMP_PRESSURE_SIZE: usize = 2;
/// Размер скорости в байтах (3 компонента i8)
pub const VELOCITY_SIZE: usize = 3;

/// Общий размер Cell в байтах (должен быть кратен выравниванию)
pub const CELL_SIZE: usize = mem::size_of::<Cell>();

/// Типы материалов для вокселей
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Material {
    Air = 0,
    Vacuum = 1,
    Stone = 2,
    Dirt = 3,
    Sand = 4,
    Water = 5,
    Ice = 6,
    Steam = 7,
    Lava = 8,
    Magma = 9,
    Wood = 10,
    Leaves = 11,
    Metal = 12,
    Plasma = 13,
    // Резерв для будущих материалов (до 255)
}

impl Default for Material {
    fn default() -> Self {
        Material::Air
    }
}

impl Material {
    /// Проверка, является ли материал твёрдым
    #[inline]
    pub const fn is_solid(&self) -> bool {
        matches!(
            self,
            Material::Stone
                | Material::Dirt
                | Material::Sand
                | Material::Ice
                | Material::Magma
                | Material::Wood
                | Material::Metal
        )
    }

    /// Проверка, является ли материал жидкостью
    #[inline]
    pub const fn is_fluid(&self) -> bool {
        matches!(self, Material::Water | Material::Lava)
    }

    /// Проверка, является ли материал газом
    #[inline]
    pub const fn is_gas(&self) -> bool {
        matches!(self, Material::Steam | Material::Plasma)
    }

    /// Проверка, может ли материал участвовать в фазовых переходах
    #[inline]
    pub const fn can_change_phase(&self) -> bool {
        matches!(
            self,
            Material::Water | Material::Ice | Material::Steam | Material::Lava | Material::Magma
        )
    }
}

/// Битовые флаги для быстрого доступа к свойствам ячейки
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellFlags(pub u8);

impl CellFlags {
    pub const NONE: Self = Self(0b0000_0000);
    pub const SOLID: Self = Self(0b0000_0001);
    pub const FLUID: Self = Self(0b0000_0010);
    pub const GAS: Self = Self(0b0000_0100);
    pub const HOT: Self = Self(0b0000_1000);   // Температура выше порога
    pub const COLD: Self = Self(0b0001_0000);  // Температура ниже порога
    pub const PRESSURIZED: Self = Self(0b0010_0000); // Высокое давление
    pub const MOVING: Self = Self(0b0100_0000); // Имеет ненулевую скорость

    #[inline]
    pub const fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    #[inline]
    pub fn set(&mut self, flag: Self, value: bool) {
        if value {
            self.0 |= flag.0;
        } else {
            self.0 &= !flag.0;
        }
    }
}

/// Основная структура ячейки воксельного мира.
/// 
/// # Память
/// Структура имеет фиксированный размер и выровнена для эффективного доступа.
/// Все поля используют квантованные значения для минимизации размера.
#[repr(C)]
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Cell {
    /// Тип материала (1 байт)
    pub material: u8,
    
    /// Битовые флаги свойств (1 байт)
    pub flags: u8,
    
    /// Температура в квантованных единицах (-32768..32767)
    /// Реальная температура = temperature * 0.01 Kelvin
    pub temperature: i16,
    
    /// Давление в квантованных единицах
    /// Реальное давление = pressure * 10 Pascal
    pub pressure: i16,
    
    /// Скорость по осям X, Y, Z в квантованных единицах
    /// Реальная скорость = velocity * 0.1 m/s
    pub velocity_x: i8,
    pub velocity_y: i8,
    pub velocity_z: i8,
    
    /// Глубина жидкости (для алгоритма DHIMMS)
    /// 0 = нет жидкости, 255 = максимальная глубина
    pub fluid_depth: u8,
    
    /// Резервные байты для выравнивания (до 16 байт total)
    _padding: [u8; 3],
}

// Проверка размера на этапе компиляции
const _: () = assert!(CELL_SIZE == 16, "Cell должен быть ровно 16 байт");

impl Default for Cell {
    fn default() -> Self {
        Self::AIR
    }
}

impl Cell {
    /// Пустая ячейка (воздух)
    pub const AIR: Self = Self {
        material: Material::Air as u8,
        flags: CellFlags::NONE.0,
        temperature: 2930, // 293.0 K = 20°C
        pressure: 10132,   // 101320 Pa = 1 атм
        velocity_x: 0,
        velocity_y: 0,
        velocity_z: 0,
        fluid_depth: 0,
        _padding: [0; 3],
    };

    /// Ячейка из вакуума
    pub const VACUUM: Self = Self {
        material: Material::Vacuum as u8,
        flags: CellFlags::NONE.0,
        temperature: 0,
        pressure: 0,
        velocity_x: 0,
        velocity_y: 0,
        velocity_z: 0,
        fluid_depth: 0,
        _padding: [0; 3],
    };

    /// Создать новую ячейку с заданным материалом
    #[inline]
    pub const fn new(material: Material) -> Self {
        let mut flags = CellFlags::NONE;
        
        // Установить начальные флаги на основе материала
        if material.is_solid() {
            flags = CellFlags::SOLID;
        } else if material.is_fluid() {
            flags = CellFlags::FLUID;
        } else if material.is_gas() {
            flags = CellFlags::GAS;
        }

        Self {
            material: material as u8,
            flags: flags.0,
            temperature: 2930,
            pressure: 10132,
            velocity_x: 0,
            velocity_y: 0,
            velocity_z: 0,
            fluid_depth: 0,
            _padding: [0; 3],
        }
    }

    /// Получить материал как enum
    #[inline]
    pub const fn get_material(&self) -> Material {
        match self.material {
            0 => Material::Air,
            1 => Material::Vacuum,
            2 => Material::Stone,
            3 => Material::Dirt,
            4 => Material::Sand,
            5 => Material::Water,
            6 => Material::Ice,
            7 => Material::Steam,
            8 => Material::Lava,
            9 => Material::Magma,
            10 => Material::Wood,
            11 => Material::Leaves,
            12 => Material::Metal,
            13 => Material::Plasma,
            _ => Material::Air,
        }
    }

    /// Установить материал и обновить флаги
    #[inline]
    pub fn set_material(&mut self, material: Material) {
        self.material = material as u8;
        
        let mut flags = CellFlags(self.flags);
        flags.set(CellFlags::SOLID, material.is_solid());
        flags.set(CellFlags::FLUID, material.is_fluid());
        flags.set(CellFlags::GAS, material.is_gas());
        self.flags = flags.0;
    }

    /// Получить температуру в Кельвинах
    #[inline]
    pub const fn get_temperature(&self) -> f32 {
        self.temperature as f32 * 0.01
    }

    /// Установить температуру в Кельвинах (квантуется)
    #[inline]
    pub fn set_temperature(&mut self, temp_kelvin: f32) {
        self.temperature = (temp_kelvin * 100.0).clamp(-32768.0, 32767.0) as i16;
        
        // Обновить флаги HOT/COLD
        let mut flags = CellFlags(self.flags);
        flags.set(CellFlags::HOT, self.temperature > 3730); // > 373K = > 100°C
        flags.set(CellFlags::COLD, self.temperature < 2730); // < 273K = < 0°C
        self.flags = flags.0;
    }

    /// Получить давление в Паскалях
    #[inline]
    pub const fn get_pressure(&self) -> f32 {
        self.pressure as f32 * 10.0
    }

    /// Установить давление в Паскалях (квантуется)
    #[inline]
    pub fn set_pressure(&mut self, pressure_pa: f32) {
        self.pressure = (pressure_pa / 10.0).clamp(-32768.0, 32767.0) as i16;
        
        let mut flags = CellFlags(self.flags);
        flags.set(CellFlags::PRESSURIZED, self.pressure > 20000); // > 2 атм
        self.flags = flags.0;
    }

    /// Получить скорость как вектор (м/с)
    #[inline]
    pub fn get_velocity(&self) -> glam::Vec3 {
        glam::Vec3::new(
            self.velocity_x as f32 * 0.1,
            self.velocity_y as f32 * 0.1,
            self.velocity_z as f32 * 0.1,
        )
    }

    /// Установить скорость (м/с, квантуется)
    #[inline]
    pub fn set_velocity(&mut self, vel: glam::Vec3) {
        self.velocity_x = (vel.x * 10.0).clamp(-127.0, 127.0) as i8;
        self.velocity_y = (vel.y * 10.0).clamp(-127.0, 127.0) as i8;
        self.velocity_z = (vel.z * 10.0).clamp(-127.0, 127.0) as i8;
        
        let has_velocity = self.velocity_x != 0 || self.velocity_y != 0 || self.velocity_z != 0;
        let mut flags = CellFlags(self.flags);
        flags.set(CellFlags::MOVING, has_velocity);
        self.flags = flags.0;
    }

    /// Проверка, пуста ли ячейка (воздух или вакуум)
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.material <= Material::Vacuum as u8
    }

    /// Проверка, твёрдая ли ячейка
    #[inline]
    pub const fn is_solid(&self) -> bool {
        CellFlags(self.flags).has(CellFlags::SOLID)
    }

    /// Проверка, жидкость ли это
    #[inline]
    pub const fn is_fluid(&self) -> bool {
        CellFlags(self.flags).has(CellFlags::FLUID)
    }

    /// Проверка, газ ли это
    #[inline]
    pub const fn is_gas(&self) -> bool {
        CellFlags(self.flags).has(CellFlags::GAS)
    }

    /// Проверка, движется ли ячейка
    #[inline]
    pub const fn is_moving(&self) -> bool {
        CellFlags(self.flags).has(CellFlags::MOVING)
    }

    /// Получить квадрат скорости (для оптимизации столкновений)
    #[inline]
    pub const fn velocity_squared(&self) -> i32 {
        (self.velocity_x as i32).pow(2)
            + (self.velocity_y as i32).pow(2)
            + (self.velocity_z as i32).pow(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_size() {
        assert_eq!(CELL_SIZE, 16);
    }

    #[test]
    fn test_cell_default() {
        let cell = Cell::default();
        assert_eq!(cell.get_material(), Material::Air);
        assert!(!cell.is_solid());
    }

    #[test]
    fn test_cell_material() {
        let mut cell = Cell::new(Material::Stone);
        assert!(cell.is_solid());
        
        cell.set_material(Material::Water);
        assert!(cell.is_fluid());
        assert!(!cell.is_solid());
    }

    #[test]
    fn test_temperature_quantization() {
        let mut cell = Cell::default();
        cell.set_temperature(293.15); // 20°C
        assert!((cell.get_temperature() - 293.15).abs() < 0.02);
    }

    #[test]
    fn test_velocity_squared() {
        let mut cell = Cell::default();
        cell.set_velocity(glam::Vec3::new(1.0, 2.0, 3.0));
        // 10^2 + 20^2 + 30^2 = 100 + 400 + 900 = 1400
        assert_eq!(cell.velocity_squared(), 1400);
    }
}
