use std::fs;
use std::path::Path;
use std::error::Error;
use std::io::{Read, BufReader};
use radiance::math::Mat44;
use byteorder::{LittleEndian, ReadBytesExt};
use super::read_vec;

#[derive(Debug)]
pub struct VertexComponent(u32);
impl VertexComponent {
    pub const Position: Self = VertexComponent(0b1);
    pub const Unknown2: Self = VertexComponent(0b10);
    pub const Unknown4: Self = VertexComponent(0b100);
    pub const Unknown8: Self = VertexComponent(0b1000);
    pub const TexCoord: Self = VertexComponent(0b10000);
    pub const Unknown20: Self = VertexComponent(0b100000);
    pub const Unknown40: Self = VertexComponent(0b1000000);
    pub const Unknown80: Self = VertexComponent(0b10000000);
    pub const Unknown100: Self = VertexComponent(0b100000000);

    pub fn has(&self, c: VertexComponent) -> bool {
        (self.0 & c.0) != 0
    }
}

#[derive(Debug)]
pub struct PolVertexPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug)]
pub struct PolVertexTexCoord {
    pub u: f32,
    pub v: f32,
}

#[derive(Debug)]
pub struct PolVertex {
    pub position: PolVertexPosition,
    pub unknown2: Option<[f32; 3]>,
    pub unknown4: Option<[f32; 1]>,
    pub unknown8: Option<[f32; 1]>,
    pub tex_coord: PolVertexTexCoord,
    pub unknown20: Option<[f32; 2]>,
    pub unknown40: Option<[f32; 2]>,
    pub unknown80: Option<[f32; 2]>,
    pub unknown100: Option<[f32; 4]>,
}

#[derive(Debug)]
pub struct PolMaterialInfo {
    pub unknown_dw0: u32,
    pub unknown_68: Vec<u8>,
    pub unknown_float: f32,
    pub light_map_count: u32,
    pub light_map_names: Vec<String>,
}

#[derive(Debug)]
pub struct PolTriangle {
    pub indices: [u16; 3],
}

#[derive(Debug)]
pub struct PolMesh {
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
    pub vertex_type: VertexComponent,
    pub vertex_count: u32,
    pub vertices: Vec<PolVertex>,
    pub material_info_count: u32,
    pub material_info: Vec<PolMaterialInfo>,
    pub unknown2: u32,
    pub unknown3: u32,
    pub unknown4: u32,
    pub triangle_count: u32,
    pub triangles: Vec<PolTriangle>,
}

#[derive(Debug)]
pub struct UnknownData {
    pub unknown: Vec<u8>, // size: 32
    pub matrix: Mat44,
    pub unknown2: u32,
    pub str_len: u32,
    pub ddd_str: Vec<u8>,
}

#[derive(Debug)]
pub struct GeomNodeDesc {
    pub unknown: Vec<u8>, // size: 52
}

#[derive(Debug)]
pub struct PolFile {
    pub magic: [u8; 4],
    pub some_flag: u32,
    pub mesh_count: u32,
    pub geom_node_descs: Vec<GeomNodeDesc>,
    pub unknown_count: u32,
    pub unknown_data: Vec<UnknownData>,
    pub meshes: Vec<PolMesh>,
}

pub fn pol_load_from_file<P: AsRef<Path>>(path: P) -> Result<PolFile, Box<dyn Error>> {
    let mut reader = BufReader::new(fs::File::open(path)?);
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;

    match magic {
        [0x50, 0x4f, 0x4c, 0x59] => (), // "POLY"
        _ => panic!("Not a valid pol file"),
    }

    let some_flag = reader.read_u32::<LittleEndian>()?;
    let mesh_count = reader.read_u32::<LittleEndian>()?;
    let mut geom_node_descs = vec![];
    for i in 0..mesh_count {
        let unknown = read_vec(&mut reader, 52)?;
        geom_node_descs.push(GeomNodeDesc {
            unknown,
        });
    }

    let mut unknown_count = 0;
    let mut unknown_data = vec![];
    if some_flag > 100 {
        unknown_count = reader.read_u32::<LittleEndian>()?;
        if unknown_count > 0 {
            for i in 0..unknown_count {
                let u = read_vec(&mut reader, 32)?;
                let mut mat = Mat44::new_zero();
                reader.read_f32_into::<LittleEndian>(unsafe {
                    std::mem::transmute::<&mut [[f32; 4]; 4], &mut [f32; 16]>(mat.floats_mut())
                })?;
                let u2 = reader.read_u32::<LittleEndian>()?;
                let str_len = reader.read_u32::<LittleEndian>()?;
                let ddd_str = read_vec(&mut reader, str_len as usize)?;
                unknown_data.push(UnknownData {
                    unknown: u,
                    matrix: mat,
                    unknown2: u2,
                    ddd_str,
                    str_len,
                })
            }
        } 
    }

    let mut meshes = vec![];
    for i in 0..mesh_count {
        meshes.push(read_pol_mesh(&mut reader)?);
    }

    Ok(PolFile {
        magic,
        some_flag,
        mesh_count,
        geom_node_descs,
        unknown_count,
        unknown_data,
        meshes,
    })
}

fn read_pol_mesh(reader: &mut dyn Read) -> Result<PolMesh, Box<dyn Error>> {
    let mut aabb_min = [0f32; 3];
    let mut aabb_max = [0f32; 3];
    reader.read_f32_into::<LittleEndian>(&mut aabb_min)?;
    reader.read_f32_into::<LittleEndian>(&mut aabb_max)?;
    let vertex_type = VertexComponent { 0: reader.read_i32::<LittleEndian>()? as u32 };
    let vertex_count = reader.read_u32::<LittleEndian>()?;
    let size = calc_vertex_size(vertex_type.0 as i32);
    let mut vertices = vec![];
    for i in 0..vertex_count {
        if !vertex_type.has(VertexComponent::Position) {
            panic!("This POL file doesn't have position info, which doesn't support currently.");
        }

        if !vertex_type.has(VertexComponent::TexCoord) {
            panic!("This POL file doesn't have texture coord info, which doesn't support currently.");
        }

        let position = PolVertexPosition {
            x: reader.read_f32::<LittleEndian>()?,
            y: reader.read_f32::<LittleEndian>()?,
            z: reader.read_f32::<LittleEndian>()?,
        };

        let unknown2 = if vertex_type.has(VertexComponent::Unknown2) {
            let mut arr = [0.; 3];
            reader.read_f32_into::<LittleEndian>(&mut arr);
            Some(arr)
        } else {
            None
        };

        let unknown4 = if vertex_type.has(VertexComponent::Unknown4) {
            let mut arr = [0.; 1];
            reader.read_f32_into::<LittleEndian>(&mut arr)?;
            Some(arr)
        } else {
            None
        };
        
        let unknown8 = if vertex_type.has(VertexComponent::Unknown8) {
            let mut arr = [0.; 1];
            reader.read_f32_into::<LittleEndian>(&mut arr)?;
            Some(arr)
        } else {
            None
        };

        let tex_coord = PolVertexTexCoord {
            u: reader.read_f32::<LittleEndian>()?,
            v: reader.read_f32::<LittleEndian>()?,
        };

        let unknown20 = if vertex_type.has(VertexComponent::Unknown20) {
            let mut arr = [0.; 2];
            reader.read_f32_into::<LittleEndian>(&mut arr)?;
            Some(arr)
        } else {
            None
        };

        let unknown40 = if vertex_type.has(VertexComponent::Unknown40) {
            let mut arr = [0.; 2];
            reader.read_f32_into::<LittleEndian>(&mut arr)?;
            Some(arr)
        } else {
            None
        };

        let unknown80 = if vertex_type.has(VertexComponent::Unknown80) {
            let mut arr = [0.; 2];
            reader.read_f32_into::<LittleEndian>(&mut arr)?;
            Some(arr)
        } else {
            None
        };
        
        let unknown100 = if vertex_type.has(VertexComponent::Unknown100) {
            let mut arr = [0.; 4];
            reader.read_f32_into::<LittleEndian>(&mut arr)?;
            Some(arr)
        } else {
            None
        };

        vertices.push(PolVertex {
            position,
            unknown2,
            unknown4,
            unknown8,
            tex_coord,
            unknown20,
            unknown40,
            unknown80,
            unknown100,
        });
    }

    let material_info_count = reader.read_u32::<LittleEndian>()?;
    let mut material_info = vec![];
    for i in 0..material_info_count {
        let unknown_dw0 = reader.read_u32::<LittleEndian>()?;
        let unknown_68 = read_vec(reader, 64)?;
        let unknown_float = reader.read_f32::<LittleEndian>()?.min(128.).max(0.);
        let light_map_count = reader.read_u32::<LittleEndian>()?;
        let mut light_map_names = vec![];
        for j in 0..light_map_count {
            let name = read_vec(reader, 64)?;
            let name_s = String::from_utf8(name.into_iter().take_while(|&c| c != 0).collect()).unwrap();
            light_map_names.push(name_s);
        }

        material_info.push(PolMaterialInfo {
            unknown_dw0,
            unknown_68,
            unknown_float,
            light_map_count,
            light_map_names,
        });
    }

    let unknown2 = reader.read_u32::<LittleEndian>()?;
    let unknown3 = reader.read_u32::<LittleEndian>()?;
    let unknown4 = reader.read_u32::<LittleEndian>()?;
    let triangle_count = reader.read_u32::<LittleEndian>()?;
    let mut triangles = vec![];
    for i in 0..triangle_count
    {
        let mut indices = [0u16; 3];
        reader.read_u16_into::<LittleEndian>(&mut indices)?;
        triangles.push(PolTriangle {
            indices,
        });
    }

    Ok(PolMesh {
        aabb_min,
        aabb_max,
        vertex_type,
        vertex_count,
        vertices,
        material_info_count,
        material_info,
        unknown2,
        unknown3,
        unknown4,
        triangle_count,
        triangles,
    })
}

fn calc_vertex_size(t: i32) -> usize {
    if t < 0 {
        return (t & 0x7FFFFFFF) as usize;
    }

    let mut size = 0;

    if t & 1 != 0 {
        size += 12;
    }

    if t & 2 != 0 {
        size += 12;
    }

    if t & 4 != 0 {
        size += 4;
    }
    
    if t & 8 != 0 {
        size += 4;
    }
    
    if t & 0x10 != 0 {
        size += 8;
    }
    
    if t & 0x20 != 0 {
        size += 8;
    }
    
    if t & 0x40 != 0 {
        size += 8;
    }
    
    if t & 0x80 != 0 {
        size += 8;
    }
    
    if t & 0x100 != 0 {
        size += 16;
    }

    return size;
}