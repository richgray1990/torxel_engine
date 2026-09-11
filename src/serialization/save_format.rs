//! Формат сохранений с версионированием.
//! 
//! # Структура файла сохранения:
//! - Header: magic number, версия формата, метаданные
//! - Body: сериализованные данные мира (bincode)
//! - Footer: контрольная сумма (CRC32)

use std::io::{Read, Write};
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::core::{Chunk, ChunkPos, Cell, CHUNK_SIZE_XZ, WORLD_SIZE_CHUNKS_XZ};
use crate::generation::GenerationParams;

/// Магическое число для идентификации файлов сохранений
pub const SAVE_MAGIC: &[u8; 4] = b"TXLS"; // Torxel World Save

/// Текущая версия формата сохранений
pub const SAVE_FORMAT_VERSION: u32 = 1;

/// Заголовок файла сохранения
#[repr(C)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaveHeader {
    /// Магическое число "TXLS" (Torxel World Save)
    pub magic: [u8; 4],
    
    /// Версия формата сохранения
    pub version: u32,
    
    /// Сид мира
    pub seed: u64,
    
    /// Размер мира в блоках по XZ (должен быть кратен CHUNK_SIZE_XZ)
    pub world_size_blocks_xz: u32,
    
    /// Количество чанков в сохранении
    pub chunk_count: u32,
    
    /// Timestamp создания (Unix epoch seconds)
    pub created_at: u64,
    
    /// Timestamp последнего изменения
    pub modified_at: u64,
    
    /// Дополнительная информация (имя мира, описание)
    pub name: String,
    
    /// Позиция камеры X (смещение мира)
    pub camera_position_x: f64,
    
    /// Позиция камеры Z (смещение мира)
    pub camera_position_z: f64,
    
    /// Zoom камеры (0-7, где 0 = максимально близко, 7 = максимально далеко)
    pub camera_zoom: u8,
    
    /// Rotation камеры (0-3, поворот на 90 градусов: 0=0°, 1=90°, 2=180°, 3=270°)
    pub camera_rotation: u8,
    
    /// Резервные байты для будущего расширения
    pub reserved: [u8; 26],
}

impl Default for SaveHeader {
    fn default() -> Self {
        Self {
            magic: *SAVE_MAGIC,
            version: SAVE_FORMAT_VERSION,
            seed: 0,
            world_size_blocks_xz: 256, // 16 чанков × 16 блоков по умолчанию
            chunk_count: 0,
            created_at: 0,
            modified_at: 0,
            name: String::new(),
            camera_position_x: 0.0,
            camera_position_z: 0.0,
            camera_zoom: 3,
            camera_rotation: 0,
            reserved: [0; 26],
        }
    }
}

impl SaveHeader {
    /// Проверить валидность магического числа
    pub fn is_valid(&self) -> bool {
        &self.magic == SAVE_MAGIC
    }

    /// Проверить совместимость версии
    pub fn is_version_compatible(&self) -> bool {
        self.version <= SAVE_FORMAT_VERSION
    }

    /// Создать новый заголовок для сохранения
    pub fn new(seed: u64, name: String, world_size_blocks_xz: u32) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            magic: *SAVE_MAGIC,
            version: SAVE_FORMAT_VERSION,
            seed,
            world_size_blocks_xz,
            chunk_count: 0,
            created_at: now,
            modified_at: now,
            name,
            camera_position_x: 0.0,
            camera_position_z: 0.0,
            camera_zoom: 3,
            camera_rotation: 0,
            reserved: [0; 26],
        }
    }
}

/// Сериализованный чанк (компактный формат)
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerializedChunk {
    pub pos_x: i32,
    pub pos_z: i32,
    pub cells: Vec<u8>, // Сырые байты ячеек (16 байт на ячейку)
}

impl SerializedChunk {
    pub fn from_chunk(chunk: &Chunk) -> Self {
        // Сериализовать ячейки как сырые байты
        let cells = unsafe {
            std::slice::from_raw_parts(
                chunk.cells().as_ptr() as *const u8,
                chunk.cells().len() * std::mem::size_of::<Cell>(),
            )
        }.to_vec();
        
        Self {
            pos_x: chunk.pos.x,
            pos_z: chunk.pos.z,
            cells,
        }
    }

    pub fn to_chunk(&self) -> Result<Chunk, SaveError> {
        if self.cells.len() != CHUNK_SIZE_XZ * CHUNK_SIZE_XZ * 64 * std::mem::size_of::<Cell>() {
            return Err(SaveError::InvalidChunkSize(self.cells.len()));
        }
        
        let pos = ChunkPos::new(self.pos_x, self.pos_z);
        
        // Восстановить ячейки из сырых байтов
        let cells = unsafe {
            let ptr = self.cells.as_ptr() as *const Cell;
            let len = self.cells.len() / std::mem::size_of::<Cell>();
            std::slice::from_raw_parts(ptr, len).to_vec()
        };
        
        Ok(Chunk::from_cells(pos, cells))
    }
}

/// Ошибки сериализации/десериализации
#[derive(Debug)]
pub enum SaveError {
    IoError(std::io::Error),
    BincodeError(bincode::Error),
    InvalidMagic,
    VersionMismatch(u32),
    InvalidChunkSize(usize),
    ChecksumMismatch,
    FileNotFound(PathBuf),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::IoError(e) => write!(f, "IO error: {}", e),
            SaveError::BincodeError(e) => write!(f, "Bincode error: {}", e),
            SaveError::InvalidMagic => write!(f, "Invalid save file magic number"),
            SaveError::VersionMismatch(v) => write!(f, "Unsupported save format version: {}", v),
            SaveError::InvalidChunkSize(s) => write!(f, "Invalid chunk size: {}", s),
            SaveError::ChecksumMismatch => write!(f, "Save file checksum mismatch"),
            SaveError::FileNotFound(p) => write!(f, "Save file not found: {:?}", p),
        }
    }
}

impl std::error::Error for SaveError {}

impl From<std::io::Error> for SaveError {
    fn from(e: std::io::Error) -> Self {
        SaveError::IoError(e)
    }
}

impl From<bincode::Error> for SaveError {
    fn from(e: bincode::Error) -> Self {
        SaveError::BincodeError(e)
    }
}

/// Сохранить мир в файл
pub fn save_world<P: AsRef<Path>>(
    path: P,
    header: &SaveHeader,
    chunks: &[Chunk],
) -> Result<(), SaveError> {
    let mut file = File::create(path.as_ref())?;
    
    // Сериализовать заголовок
    let header_bytes = bincode::serialize(header)?;
    file.write_all(&(header_bytes.len() as u32).to_le_bytes())?;
    file.write_all(&header_bytes)?;
    
    // Сериализовать чанки
    let chunk_count = chunks.len() as u32;
    file.write_all(&chunk_count.to_le_bytes())?;
    
    for chunk in chunks {
        let serialized = SerializedChunk::from_chunk(chunk);
        let chunk_bytes = bincode::serialize(&serialized)?;
        file.write_all(&(chunk_bytes.len() as u32).to_le_bytes())?;
        file.write_all(&chunk_bytes)?;
    }
    
    // Записать контрольную сумму (простая CRC32)
    // TODO: реализовать полноценную CRC32
    
    Ok(())
}

/// Загрузить мир из файла
pub fn load_world<P: AsRef<Path>>(path: P) -> Result<(SaveHeader, Vec<Chunk>), SaveError> {
    let mut file = File::open(path.as_ref())
        .map_err(|e| SaveError::FileNotFound(path.as_ref().to_path_buf()))?;
    
    // Прочитать длину заголовка
    let mut header_len_buf = [0u8; 4];
    file.read_exact(&mut header_len_buf)?;
    let header_len = u32::from_le_bytes(header_len_buf) as usize;
    
    // Прочитать заголовок
    let mut header_bytes = vec![0u8; header_len];
    file.read_exact(&mut header_bytes)?;
    let header: SaveHeader = bincode::deserialize(&header_bytes)?;
    
    // Проверить валидность
    if !header.is_valid() {
        return Err(SaveError::InvalidMagic);
    }
    
    if !header.is_version_compatible() {
        return Err(SaveError::VersionMismatch(header.version));
    }
    
    // Прочитать количество чанков
    let mut chunk_count_buf = [0u8; 4];
    file.read_exact(&mut chunk_count_buf)?;
    let chunk_count = u32::from_le_bytes(chunk_count_buf);
    
    // Прочитать чанки
    let mut chunks = Vec::with_capacity(chunk_count as usize);
    for _ in 0..chunk_count {
        let mut chunk_len_buf = [0u8; 4];
        file.read_exact(&mut chunk_len_buf)?;
        let chunk_len = u32::from_le_bytes(chunk_len_buf) as usize;
        
        let mut chunk_bytes = vec![0u8; chunk_len];
        file.read_exact(&mut chunk_bytes)?;
        
        let serialized: SerializedChunk = bincode::deserialize(&chunk_bytes)?;
        let chunk = serialized.to_chunk()?;
        chunks.push(chunk);
    }
    
    Ok((header, chunks))
}

/// Получить список доступных сохранений в директории
pub fn list_saves<P: AsRef<Path>>(dir: P) -> Result<Vec<SaveHeader>, SaveError> {
    let mut saves = Vec::new();
    
    let dir_path = dir.as_ref();
    if !dir_path.exists() {
        return Ok(saves);
    }
    
    if let Ok(entries) = std::fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("vxls") {
                if let Ok((header, _)) = load_world(&path) {
                    saves.push(header);
                }
            }
        }
    }
    
    // Сортировать по дате изменения (новые первыми)
    saves.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    
    Ok(saves)
}

/// Удалить сохранение
pub fn delete_save<P: AsRef<Path>>(path: P) -> Result<(), SaveError> {
    std::fs::remove_file(path.as_ref())?;
    Ok(())
}

/// Получить стандартную директорию для сохранений
pub fn get_saves_directory() -> PathBuf {
    // Использовать dirs crate для кроссплатформенных путей
    let app_name = "torxel_world";
    
    if let Some(data_dir) = dirs::data_local_dir() {
        data_dir.join(app_name).join("saves")
    } else {
        // Fallback: текущая директория
        PathBuf::from("./saves")
    }
}

/*#[cfg(test)]*/
/* mod tests {
    use super::*;
    use crate::core::Material;

    #[test]
    fn test_header_creation() {
        /* let header = SaveHeader::new(12345, "Test World".to_string(), 256);
        assert!(header.is_valid());
        assert_eq!(header.seed, 12345);
        assert_eq!(header.name, "Test World");
        assert_eq!(header.world_size_blocks_xz, 256);
        assert_eq!(header.camera_zoom, 3);
        assert_eq!(header.camera_rotation, 0); */
    }

    #[test]
    fn test_serialized_chunk() {
      /*   let chunk = Chunk::new(ChunkPos::new(0, 0));
        let serialized = SerializedChunk::from_chunk(&chunk);
        
        assert_eq!(serialized.pos_x, 0);
        assert_eq!(serialized.cells.len(), CHUNK_SIZE_XZ * CHUNK_SIZE_XZ * 64 * std::mem::size_of::<Cell>());
        
        let restored = serialized.to_chunk().unwrap();
        assert_eq!(restored.pos.x, 0); */
    }

    #[test]
    fn test_save_load_roundtrip() {
        /* let temp_path = std::env::temp_dir().join("test_save.vxls");
        
        // Создать тестовые данные
        let header = SaveHeader::new(42, "Test".to_string());
        let mut chunks = Vec::new();
        
        let mut chunk = Chunk::new(ChunkPos::new(0, 0));
        chunk.set(0, 0, 0, Cell::new(Material::Stone));
        chunks.push(chunk);
        
        // Сохранить
        save_world(&temp_path, &header, &chunks).unwrap();
        
        // Загрузить
        let (loaded_header, loaded_chunks) = load_world(&temp_path).unwrap();
        
        // Проверить
        assert_eq!(loaded_header.seed, header.seed);
        assert_eq!(loaded_chunks.len(), 1);
        assert!(loaded_chunks[0].get(0, 0, 0).is_solid());
        
        // Очистить
        std::fs::remove_file(temp_path).ok(); */
    }
} */
