//! Voxel World Engine Core
//! 
//! Zero-Allocation ядро данных для воксельного движка.
//! Все структуры оптимизированы для производительности:
//! - #[repr(C)] для предсказуемой памяти
//! - Плоские Vec<Cell> без аллокаций в горячих путях
//! - Битовые флаги и квантованные значения

pub mod cell;
pub mod chunk;
pub mod world;

pub use cell::*;
pub use chunk::*;
pub use world::*;
