extern crate image;
extern crate indicatif;
extern crate nalgebra;

type Scene = Vec<Mesh>;

struct Mesh {
    position: Vector,
    geometry: Geometry,
    material: Material,
}

type Float = f32;

type Vector = nalgebra::Vector3<Float>;

enum Geometry {
    Sphere { radius: Float },
}

struct Material {
    color: Color,
}

#[derive(Clone)]
struct Color {
    red: Float,
    green: Float,
    blue: Float,
}

struct Camera {
    eye: Point,
    target: Point,
    aspect: Float,
    fovy: Float,
    znear: Float,
    zfar: Float,
}

type Point = nalgebra::Point3<Float>;

struct RendererSettings {
    definition: Integer,
    anti_aliasing: Integer,
}

type Integer = u32;

impl Mesh {
    fn distance(&self, point: &Point) -> Float {
        match self.geometry {
            Geometry::Sphere { radius } => (point - &self.position).coords.norm() - radius,
        }
    }
}

impl Color {
    fn new(red: Float, green: Float, blue: Float) -> Color {
        Color { red, green, blue }
    }

    fn to_pixel(&self) -> [u8; 3] {
        return [
            (self.red * 255.0) as u8,
            (self.green * 255.0) as u8,
            (self.blue * 255.0) as u8,
        ];
    }
}

fn render(scene: &Scene, camera: &Camera, settings: &RendererSettings) {
    let projection_matrix =
        nalgebra::Perspective3::new(camera.aspect, camera.fovy, camera.znear, camera.zfar);
    // println!("projection: {}", projection_matrix.as_matrix());
    let view_matrix =
        nalgebra::Isometry3::look_at_rh(&camera.eye, &camera.target, &nalgebra::Vector3::y());
    // println!("view: {}", view_matrix);
    // for mesh in scene.iter() {
    //     let model = mesh.position.to_na_translation();
    //     println!("model: {}", model)
    // }
    let height = (settings.definition * settings.anti_aliasing) as Float;
    let width = height * camera.aspect;
    // println!("width: {} height: {}", width, height);
    let mut pixels: Vec<u8> = Vec::new();
    pixels.reserve((width * height) as usize);
    let bar = indicatif::ProgressBar::new(height as u64);
    bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {msg} {bar:40.cyan/blue} [ETA: {eta}]")
            .progress_chars("##-"),
    );
    bar.set_message("Rendering");
    for y in 0..(height as Integer) {
        for x in 0..(width as Integer) {
            let mvp_x = 2.0 * (x as Float) / width - 1.0;
            let mvp_y = 1.0 - 2.0 * (y as Float) / height;
            let origin = view_matrix.inverse_transform_point(
                &projection_matrix.unproject_point(&nalgebra::Point3::new(mvp_x, mvp_y, -1.0)),
            );
            let target = view_matrix.inverse_transform_point(
                &projection_matrix.unproject_point(&nalgebra::Point3::new(mvp_x, mvp_y, 1.0)),
            );
            // println!("from {} to {}", origin, target);
            let max_t = (target - origin).norm();
            let direction = (target - origin).normalize();
            let color = march_ray(origin, direction, max_t, scene);
            // println!("{}", ray);
            // println!("{}x{}", near_screen.x, near_screen.y);
            for channel in color.to_pixel().iter() {
                // for channel in color.to_array().iter() {
                pixels.push(*channel);
            }
        }
        bar.inc(1);
    }
    bar.set_message("Saving");
    bar.enable_steady_tick(13);
    // println!("pixels: {}", pixels.len());
    image::ImageRgb8(image::ImageBuffer::from_raw(width as u32, height as u32, pixels).unwrap())
        .resize(
            (width / settings.anti_aliasing as f32) as u32,
            (height / settings.anti_aliasing as f32) as u32,
            image::FilterType::Gaussian,
        )
        .save("render.png")
        .unwrap();
    bar.finish();
}

fn march_ray(origin: Point, direction: Vector, max_t: Float, scene: &Scene) -> Color {
    let mut t = 0.0;
    while t < max_t {
        // println!("t = {}", t);
        let point = origin + t * direction;
        // println!("point = {}", point);
        let (mesh, distance) = scene
            .iter()
            .map(|mesh| (mesh, mesh.distance(&point)))
            .min_by(|(_mesh1, distance1), (_mesh2, distance2)| {
                distance1.partial_cmp(distance2).unwrap()
            })
            .unwrap();
        // println!("distance = {}", distance);
        if distance < 0.1 {
            return mesh.material.color.clone();
        }
        t += distance;
    }
    Color::new(0.2, 0.2, 0.2)
}

fn main() {
    let scene = vec![Mesh {
        position: nalgebra::Vector3::new(3.0, 2.0, -10.0),
        geometry: Geometry::Sphere { radius: 3.0 },
        material: Material {
            color: Color::new(1.0, 0.0, 0.0),
        },
    }];
    let camera = Camera {
        eye: Point::new(0.0, 0.0, 10.0),
        target: Point::new(0.0, 0.0, 0.0),
        aspect: 3.0 / 2.0,
        fovy: 3.14 / 4.0,
        znear: 1.0,
        zfar: 1000.0,
    };
    let settings = RendererSettings {
        definition: 100,
        anti_aliasing: 2,
    };
    render(&scene, &camera, &settings)
}
