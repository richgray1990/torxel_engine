# Архитектура Voxel World Engine

## Обзор

Voxel World Engine — это высокопроизводительный воксельный движок с акцентом на:
- **Zero-Allocation**: Никаких аллокаций памяти в горячих путях симуляции
- **Toroidal Topology**: Мир без границ (циклическая топология тора)
- **ECS Architecture**: Логика разделена на независимые системы Bevy
- **Data-Oriented Design**: Оптимизация для кэш-локальности и SIMD

## Структура данных

### Cell (16 байт)

```
┌──────────┬──────────┬─────────────┬─────────────┬──────────┬──────────┬──────────┬──────────┬─────────────┐
│ material │  flags   │ temperature │  pressure   │ vel_x    │ vel_y    │ vel_z    │ fluid_dp │  _padding   │
│  u8      │   u8     │    i16      │    i16      │  i8      │  i8      │  i8      │  u8      │  [u8; 3]    │
└──────────┴──────────┴─────────────┴─────────────┴──────────┴──────────┴──────────┴──────────┴─────────────┘
 0          1          2             4             6          7          8          9          10-12
```

**Оптимизации:**
- Квантование: температура (0.01K), давление (10Pa), скорость (0.1 m/s)
- Битовые флаги для быстрого доступа к свойствам
- Фиксированный размер для предсказуемости памяти

### Chunk (32³ = 32,768 ячеек = 512 KB)

```rust
struct Chunk {
    cells: Vec<Cell>,        // Плоский массив, row-major order
    pos: ChunkPos,           // Позиция в мире
    is_dirty: bool,          // Флаг модификации
    active_cell_count: u32,  // Статистика
}
```

**Индексация:**
```
index = x + y * SIZE + z * SIZE²  // row-major order
```

**Торический wrapping (для SIZE = степень двойки):**
```
wrapped_index = index & (SIZE - 1)  // Быстрый модуль через bitmask
```

### World (16³ чанков = 4,096 чанков = 2 GB максимум)

```rust
struct ChunkManager {
    chunks: HashMap<u64, Chunk>,  // Активные чанки
    chunk_pool: Vec<Chunk>,       // Пул для переиспользования
    world_size_chunks: usize,     // Размер мира
    stats: ChunkManagerStats,     // Статистика
}
```

## Торическая математика

### Обёртка координат

```rust
// Общий случай
wrapped = ((coord % size) + size) % size

// Для степеней двойки (оптимизация)
wrapped = coord & (size - 1)
```

### Расстояние на торе

```rust
fn toroidal_distance(a, b, size) -> i32 {
    let diff = (a - b).abs();
    diff.min(size - diff)  // Минимальное расстояние через границу
}
```

## Конвейер симуляции (план для v2)

```
┌──────────────────────────────────────────────────────────────┐
│                        GAME LOOP                             │
├──────────────────────────────────────────────────────────────┤
│  1. Input Processing        (игрок, UI)                      │
│  2. Physics (DHIMMS)        (гидродинамика, 5 мс)            │
│  3. Thermodynamics          (теплопередача, 2 мс)            │
│  4. Phase Dynamics          (фазовые переходы, 2 мс)         │
│  5. Collision Detection     (столкновения, <1 мс)            │
│  6. Chunk Streaming         (загрузка/выгрузка)              │
│  7. Rendering               (меши, инстансинг, 16.67 мс)     │
└──────────────────────────────────────────────────────────────┘
                        │
                        ▼
                Target: 60 FPS (16.67 ms per frame)
```

## Управление памятью

### Стратегии

1. **Предварительное выделение**: `Vec::with_capacity(CHUNK_VOLUME)`
2. **Пул объектов**: Переиспользование чанков вместо аллокации/освобождения
3. **Арена аллокация**: Для временных данных симуляции
4. **Двойная буферизация**: Чтение из одного буфера, запись в другой

### Горячие пути (без аллокаций)

```rust
// ✅ ХОРОШО: доступ к существующей памяти
#[inline]
pub fn get(&self, x: usize, y: usize, z: usize) -> &Cell {
    &self.cells[self.index(x, y, z)]
}

// ❌ ПЛОХО: аллокация в цикле
pub fn get_neighbors(&self, x: usize, y: usize, z: usize) -> Vec<Cell> {
    // Создаёт новый Vec каждый вызов!
}

// ✅ ХОРОШО: передача буфера извне
pub fn get_neighbors_buf(&self, x: usize, y: usize, z: usize, buf: &mut [Cell; 6]) {
    // Заполняет существующий буфер
}
```

## Производительность

### Целевые метрики

| Компонент | Время | Примечания |
|-----------|-------|------------|
| Access Cell | <10 ns | Прямой доступ к массиву |
| Generate Chunk | <5 ms | 32³ ячеек с шумом |
| Physics Tick | 5-15 ms | DHIMMS алгоритм |
| Thermodynamics | 2 ms | Теплопередача между ячейками |
| Render Mesh | <16 ms | Инстансинг, frustum culling |

### Профилирование

```bash
# CPU профилирование
cargo flamegraph

# Память профилирование
cargo heaptrack

# Бенчмарки
cargo bench
```

## Расширяемость

### Добавление нового материала

1. Добавить variant в `Material` enum
2. Обновить `is_solid()`, `is_fluid()`, `is_gas()` методы
3. Добавить текстуру/цвет в рендерере
4. Определить физические свойства (плотность, теплопроводность)

### Добавление новой физической системы

1. Создать систему в отдельном модуле
2. Добавить ресурс для хранения состояния
3. Зарегистрировать в `App::add_systems()`
4. Убедиться, что укладывается в бюджет времени (см. таблицу выше)

## Тестирование

### Юнит-тесты

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_cell_size() {
        assert_eq!(CELL_SIZE, 16);
    }
    
    #[test]
    fn test_wrapping() {
        assert_eq!(wrap_coord(-1, 100), 99);
        assert_eq!(wrap_coord(100, 100), 0);
    }
}
```

### Интеграционные тесты

```rust
#[test]
fn test_world_generation() {
    let gen = TerrainGenerator::new(params);
    let chunk = gen.generate_chunk(ChunkPos::new(0, 0, 0));
    assert!(!chunk.is_empty());
}
```

## Безопасность

### Проверки во время компиляции

```rust
// Проверка размера Cell
const _: () = assert!(CELL_SIZE == 16, "Cell должен быть 16 байт");

// Проверка, что CHUNK_SIZE - степень двойки
const _: () = assert!(CHUNK_SIZE.is_power_of_two(), "...");
```

### Debug assertions

```rust
pub fn get(&self, x: usize, y: usize, z: usize) -> &Cell {
    debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
    &self.cells[self.index(x, y, z)]
}
```

## Будущие улучшения

### v2 (Физика)
- [ ] Отложенный пересчёт глубины для DHIMMS
- [ ] Оптимизация столкновений через squared velocity
- [ ] Термодинамика с теплопроводностью материалов
- [ ] Фазовые переходы (вода ↔ лёд ↔ пар)

### v3 (GPU)
- [ ] Compute shaders для термодинамики
- [ ] GPU-driven rendering
- [ ] Адаптивная симуляция (LOD для физики)

### Пост-MVP
- [ ] Мультипоточная симуляция (rayon)
- [ ] Сетевая игра (сервер-клиент архитектура)
- [ ] Моддинг API (Lua или WASM скрипты)

---

*Документация версия: 1.0 (Ядро)*
*Последнее обновление: 2024*
