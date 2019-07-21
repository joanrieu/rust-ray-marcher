type Scene = Vec<Mesh>;

struct Mesh {
    transform: Transform,
    geometry: Geometry,
    material: Material,
}

struct Transform {
    position: Vec3,
}

type Vec3 = (f32, f32, f32);

enum Geometry {
    Sphere { radius: f32 },
}

impl Geometry {
    fn distance(self, point: &Vec3) -> f32 {
        match self {
            Geometry::Sphere { radius } => radius,
        }
    }
}

struct Material {
    color: Color,
}

type Color = (u8, u8, u8);

struct Camera {
    viewPlaneNormal: Vec3,
    viewUpVector: Vec3,
    viewingReferencePoint: Vec3,
    projectionReferencePoint: Vec3,
    fieldOfView: f32,
    aspectRatio: f32,
}

struct RendererSettings {
    definition: u32,
}

fn render(scene: &Scene, camera: &Camera, settings: &RendererSettings) {
    println!("render")
}

fn main() {
    let scene = vec![Mesh {
        transform: Transform {
            position: (3.0, 2.0, -10.0),
        },
        geometry: Geometry::Sphere { radius: 1.0 },
        material: Material { color: (1, 2, 3) },
    }];
    let camera = Camera {
        viewPlaneNormal: (0.0, 0.0, -1.0),
        viewUpVector: (0.0, 1.0, 0.0),
        viewingReferencePoint: (0.0, 0.0, 10.0),
        projectionReferencePoint: (0.0, 0.0, 0.0),
        fieldOfView: 90.0,
        aspectRatio: 1.5,
    };
    let settings = RendererSettings { definition: 600 };
    render(&scene, &camera, &settings)
}
