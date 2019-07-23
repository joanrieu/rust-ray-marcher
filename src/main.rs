extern crate nalgebra;

type Scene = Vec<Mesh>;

struct Mesh {
    position: Vec3,
    geometry: Geometry,
    material: Material,
}

type Float = f32;

struct Vec3 {
    x: Float,
    y: Float,
    z: Float,
}

enum Geometry {
    Sphere { radius: Float },
}

struct Material {
    color: Color,
}

type Color = (u8, u8, u8);

struct Camera {
    eye: Vec3,
    target: Vec3,
    aspect: Float,
    fovy: Float,
    znear: Float,
    zfar: Float,
}

struct RendererSettings {
    definition: Integer,
}

type Integer = u32;

impl Mesh {
    fn distance(self, point: &Vec3) -> Float {
        match self.geometry {
            Geometry::Sphere { radius } => (point - &self.position).norm() - radius,
        }
    }
}

impl Vec3 {
    fn new(x: Float, y: Float, z: Float) -> Vec3 {
        Vec3 { x, y, z }
    }

    fn norm(&self) -> Float {
        unimplemented!()
    }

    fn to_na_vector(&self) -> nalgebra::Vector3<Float> {
        nalgebra::Vector3::new(self.x, self.y, self.z)
    }

    fn to_na_point(&self) -> nalgebra::Point3<Float> {
        nalgebra::Point3::from(self.to_na_vector())
    }

    fn to_na_translation(&self) -> nalgebra::Translation3<Float> {
        nalgebra::Translation3::from(self.to_na_vector())
    }
}

impl std::ops::Sub for &Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: &Vec3) -> Vec3 {
        unimplemented!()
    }
}

fn render(scene: &Scene, camera: &Camera, settings: &RendererSettings) {
    let projectionMatrix =
        nalgebra::Perspective3::new(camera.aspect, camera.fovy, camera.znear, camera.zfar);
    println!("projection: {}", projectionMatrix.as_matrix());
    let viewMatrix = nalgebra::Isometry3::look_at_rh(
        &camera.eye.to_na_point(),
        &camera.target.to_na_point(),
        &nalgebra::Vector3::y(),
    );
    println!("view: {}", viewMatrix);
    for mesh in scene.iter() {
        let model = mesh.position.to_na_translation();
        println!("model: {}", model)
    }
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
        zfar: 1000.0,
    };
    let settings = RendererSettings { definition: 600 };
    render(&scene, &camera, &settings)
}
