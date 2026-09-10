//! Cell - базовая единица воксельного мира.
//! 
//! Архитектурные решения:
//! - Размер 32 байта: достаточно полей для всех систем (DHIMMS, термодинамика, фазы)
//! - excell_liquid: обязательное поле для DHIMMS (0-255, "сжатая" жидкость)
//! - Не все поля используются всеми системами (sparse data pattern)
//! - #[repr(C)] для предсказуемого расположения в памяти
//! - 2 ячейки на 64-байтную кэш-линию CPU

use std::mem;

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
    pub const HOT: Self = Self(0b0000_1000);      // Температура выше порога
    pub const COLD: Self = Self(0b0001_0000);     // Температура ниже порога
    pub const PRESSURIZED: Self = Self(0b0010_0000); // Высокое давление
    pub const MOVING: Self = Self(0b0100_0000);   // Имеет ненулевую скорость
    pub const PHASE_SOLID: Self = Self(0b1000_0000); // Твёрдая фаза (лёд, камень)

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
/// Размер: 32 байта (компромисс между функциональностью и кэш-локальностью)
/// 2 ячейки помещаются в 64-байтную кэш-линию CPU
/// 
/// # Поля для DHIMMS
/// - excell_liquid: количество "сжатой" жидкости (0-255)
/// - fluid_depth: глубина жидкости для расчёта перетока
/// - velocity: инерция потока
/// - pressure: давление для гидродинамики
/// 
/// # Поля для термодинамики
/// - temperature: температура для теплопередачи
/// - heat_capacity: теплоёмкость материала
/// - density: плотность для расчёта массы
#[repr(C)]
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Cell {
    /// Тип материала (1 байт)
    pub material: u8,
    
    /// Битовые флаги свойств (1 байт)
    pub flags: u8,
    
    /// DHIMMS: количество "сжатой" жидкости в ячейке (0-255)
    /// 0 = нет жидкости, 255 = максимально насыщенная ячейка
    /// Обязательно для алгоритма DHIMMS
    pub excell_liquid: u8,
    
    /// DHIMMS: глубина жидкости для расчёта перетока (0-255)
    /// Используется в отложенном пересчёте глубины
    pub fluid_depth: u8,
    
    /// Температура в квантованных единицах (-32768..32767)
    /// Реальная температура = temperature * 0.01 Kelvin
    /// Для термодинамики и фазовых переходов
    pub temperature: i16,
    
    /// Давление в квантованных единицах
    /// Реальное давление = pressure * 10 Pascal
    /// Для DHIMMS и термодинамики
    pub pressure: i16,
    
    /// Скорость по осям X, Y, Z в квантованных единицах
    /// Реальная скорость = velocity * 0.01 m/s
    /// Диапазон: -32768..32767 (i16) для точных физических вычислений DHIMMS
    /// Для DHIMMS: инерция потока
    pub velocity_x: i16,
    pub velocity_y: i16,
    pub velocity_z: i16,
    
    /// Плотность материала (0-255)
    /// Для расчёта массы и инерции в DHIMMS
    /// 128 = плотность воды по умолчанию
    pub density: u8,
    
    /// Теплоёмкость (0-255)
    /// Для термодинамики: сколько энергии поглощает ячейка
    /// 128 = средняя теплоёмкость
    pub heat_capacity: u8,
    
    /// Фаза материала (0-255)
    /// 0 = не определена, 1-255 = индекс фазы
    /// Для фазовой динамики (лёд/вода/пар)
    pub phase_state: u8,
    
    /// Резервные байты для выравнивания до 48 байт
    _padding: [u8; 31],
}

// Проверка размера на этапе компиляции
const _: () = assert!(mem::size_of::<Cell>() == 48, "Cell должен быть ровно 48 байт");

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
        excell_liquid: 0,
        fluid_depth: 0,
        temperature: 2930, // 293.0 K = 20°C
        pressure: 10132,   // 101320 Pa = 1 атм
        velocity_x: 0,
        velocity_y: 0,
        velocity_z: 0,
        density: 128,
        heat_capacity: 128,
        phase_state: 0,
        _padding: [0; 31],
    };

    /// Ячейка вакуума
    pub const VACUUM: Self = Self {
        material: Material::Vacuum as u8,
        flags: CellFlags::NONE.0,
        excell_liquid: 0,
        fluid_depth: 0,
        temperature: 0,
        pressure: 0,
        velocity_x: 0,
        velocity_y: 0,
        velocity_z: 0,
        density: 0,
        heat_capacity: 0,
        phase_state: 0,
        _padding: [0; 31],
    };

    /// Ячейка камня
    pub const STONE: Self = Self {
        material: Material::Stone as u8,
        flags: CellFlags::SOLID.0,
        excell_liquid: 0,
        fluid_depth: 0,
        temperature: 2930,
        pressure: 0,
        velocity_x: 0,
        velocity_y: 0,
        velocity_z: 0,
        density: 200, // высокая плотность
        heat_capacity: 200, // высокая теплоёмкость
        phase_state: 2, // жидкая фаза
        _padding: [0; 31],
    };

    /// Ячейка воды (для DHIMMS)
    pub const WATER: Self = Self {
        material: Material::Water as u8,
        flags: CellFlags::FLUID.0,
        excell_liquid: 255,
        fluid_depth: 255,
        temperature: 2930,
        pressure: 10132,
        velocity_x: 0,
        velocity_y: 0,
        velocity_z: 0,
        density: 255, // высокая плотность
        heat_capacity: 200, // высокая теплоёмкость
        phase_state: 1, // жидкая фаза
        _padding: [0; 31],
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

        let density = match material {
            Material::Air => 10,
            Material::Vacuum => 0,
            Material::Water => 255,
            Material::Lava => 250,
            Material::Steam => 5,
            Material::Plasma => 2,
            _ => 128,
        };

        let heat_capacity = match material {
            Material::Water => 200,
            Material::Ice => 150,
            Material::Steam => 100,
            Material::Lava => 180,
            Material::Metal => 50,
            _ => 128,
        };

        Self {
            material: material as u8,
            flags: flags.0,
            excell_liquid: if material.is_fluid() { 255 } else { 0 },
            fluid_depth: if material.is_fluid() { 255 } else { 0 },
            temperature: 2930,
            pressure: 10132,
            velocity_x: 0,
            velocity_y: 0,
            velocity_z: 0,
            density,
            heat_capacity,
            phase_state: 0,
            _padding: [0; 31],
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
            self.velocity_x as f32 * 0.01,
            self.velocity_y as f32 * 0.01,
            self.velocity_z as f32 * 0.01,
        )
    }

    /// Установить скорость (м/с, квантуется)
    #[inline]
    pub fn set_velocity(&mut self, vel: glam::Vec3) {
        self.velocity_x = (vel.x * 100.0).clamp(-32767.0, 32767.0) as i16;
        self.velocity_y = (vel.y * 100.0).clamp(-32767.0, 32767.0) as i16;
        self.velocity_z = (vel.z * 100.0).clamp(-32767.0, 32767.0) as i16;
        
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

    /// Проверка, твёрдая ли фаза (лёд, камень)
    #[inline]
    pub const fn is_phase_solid(&self) -> bool {
        CellFlags(self.flags).has(CellFlags::PHASE_SOLID)
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
        assert_eq!(mem::size_of::<Cell>(), 48);
    }

    #[test]
    fn test_excell_liquid() {
        let water = Cell::WATER;
        assert_eq!(water.excell_liquid, 255);
        
        let air = Cell::AIR;
        assert_eq!(air.excell_liquid, 0);
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
        // 100^2 + 200^2 + 300^2 = 10000 + 40000 + 90000 = 140000
        assert_eq!(cell.velocity_squared(), 140000);
    }
}
