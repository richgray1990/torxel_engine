use bevy::{prelude::*, shader::ShaderRef};
use bevy::render::render_resource::*;
use std::hash::Hash;

use bevy::{
    pbr::{Material, MaterialPipeline, MaterialPipelineKey},
    prelude::*, // Отсюда берется Handle
    render::{
        mesh::MeshVertexBufferLayoutRef,
        render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError},
    },
};


/// Компонент-маркер для корня мира (сдвигается при движении камеры)
#[derive(Component)]
pub struct WorldRoot;

/// Компонент позиции чанка (X, Z)
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i16,
    pub z: i16,
}

/// Компонент, хранящий handles мешей для чанка
#[derive(Component)]
pub struct ChunkMeshes {
    pub solid_mesh: Option<Handle<Mesh>>,
    pub water_mesh: Option<Handle<Mesh>>,
}

/// Ресурс для хранения загруженных текстур
#[derive(Resource)]
pub struct TextureAssets {
    pub dirt: Handle<Image>,
    pub stone: Handle<Image>,
    pub water: Handle<Image>,
    pub magma: Handle<Image>,
}

/// Ресурс состояния ввода для движения камеры
#[derive(Resource, Default)]
pub struct CameraInput {
    pub move_left: bool,
    pub move_right: bool,
    pub move_up: bool,
    pub move_down: bool,
}

/// Компонент-маркер для основной камеры
#[derive(Component)]
pub struct MainCamera;

/// Настраиваем кастомный материал для твердых блоков с обводкой
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SolidMaterial {
    #[texture(0, dimension = "2d")]
    pub texture: Handle<Image>,
    #[uniform(1)]
    pub outline_color: LinearRgba,
    #[uniform(2)]
    pub outline_thickness: f32,
}

impl Material for SolidMaterial {

    fn fragment_shader() -> ShaderRef{
        "shaders/solid_outline.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "shaders/solid_outline.wgsl".into()
    }
}
