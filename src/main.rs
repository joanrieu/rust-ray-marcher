extern crate nalgebra;

type Scene = Vec<Mesh>;

struct Mesh {
    position: Vec3,
    geometry: Geometry,
    material: Material,
}

impl Mesh {
    fn distance(self, point: &Vec3) -> f32 {
        match self.geometry {
            Geometry::Sphere { radius } => (point - self.position).norm() - radius,
        }
    }
}

type Vec3 = nalgebra::Vector3<f32>;

enum Geometry {
    Sphere { radius: f32 },
}

struct Material {
    color: Color,
}

type Color = (u8, u8, u8);

struct Camera {
    eye: Vec3,
    target: Vec3,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32
}

struct RendererSettings {
    definition: u32,
}

fn render(scene: &Scene, camera: &Camera, settings: &RendererSettings) {
    println!("render")
}

fn main() {
    let scene = vec![Mesh {
        position: Vec3::new(3.0, 2.0, -10.0),
        geometry: Geometry::Sphere { radius: 1.0 },
        material: Material { color: (1, 2, 3) },
    }];
    let camera = Camera {
    eye: Vec3::new(0.0, 0.0, -10.0),
    target: Vec3::new(0.0, 0.0, 0.0),
    aspect: 3.0 / 2.0,
    fovy: 3.14 / 4.0,
    znear: 1.0,
    zfar: 1000.0
} ;
    let settings = RendererSettings { definition: 600 };
    render(&scene, &camera, &settings)
}
