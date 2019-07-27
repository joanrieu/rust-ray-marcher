extern crate image;
extern crate indicatif;
extern crate nalgebra;

type Scene = Vec<Mesh>;

struct Mesh {
    geometry: Geometry,
    material: Material,
}

type Float = f32;

type Vector = nalgebra::Vector3<Float>;

enum Geometry {
    Sphere { position: Point, radius: Float },
    Triangle { corners: [Point; 3] },
}

#[derive(Clone, Copy)]
struct Material {
    color: Color,
}

#[derive(Clone, Copy)]
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

impl Geometry {
    fn distance(&self, point: &Point) -> Float {
        match self {
            Geometry::Sphere { position, radius } => (point - position).norm() - radius,
            Geometry::Triangle { corners } => {
                let x_axis = corners[1] - corners[0];
                let y_axis = corners[2] - corners[0];
                let z_axis = x_axis.cross(&y_axis);
                let base_to_world = nalgebra::Matrix3::from_columns(&[x_axis, y_axis, z_axis]);
                let world_to_base = base_to_world.pseudo_inverse(std::f32::EPSILON).unwrap();
                let translation = nalgebra::Translation3::from(corners[0].coords).inverse();
                let point_in_base = world_to_base * (translation * point);
                let x = if point_in_base.coords.x < 0.0 {
                    0.0
                } else {
                    point_in_base.coords.x
                };
                let y = if point_in_base.coords.y < 0.0 {
                    0.0
                } else {
                    point_in_base.coords.y
                };
                let z = 0.0;
                let w = if x + y > 1.0 { x + y } else { 1.0 };
                let projected_point_in_base =
                    nalgebra::Point3::from_homogeneous(nalgebra::Vector4::from([x, y, z, w]))
                        .unwrap();
                // TODO reproject in world to preserve norm (compared to other meshes)
                let vector_in_base = point_in_base - projected_point_in_base;
                let vector = base_to_world * vector_in_base;
                return vector.norm();
                // unimplemented!()
                // corners
                //     .iter()
                //     .map(|corner| Geometry::Sphere {
                //         position: *corner,
                //         radius: 0.1,
                //     })
                //     .map(|sphere| sphere.distance(point))
                //     .min_by(|a, b| a.partial_cmp(b).unwrap())
                //     .unwrap()
            }
        }
    }
}

impl Color {
    fn new(red: Float, green: Float, blue: Float) -> Color {
        Color { red, green, blue }
    }

    fn to_pixel(&self) -> [u8; 3] {
        [
            (self.red * 255.0) as u8,
            (self.green * 255.0) as u8,
            (self.blue * 255.0) as u8,
        ]
    }
}

fn render(scene: &Scene, camera: &Camera, settings: &RendererSettings) {
    let projection_matrix =
        nalgebra::Perspective3::new(camera.aspect, camera.fovy, camera.znear, camera.zfar);
    let view_matrix = nalgebra::Isometry3::look_at_rh(&camera.eye, &camera.target, &Vector::y());
    let height = (settings.definition * settings.anti_aliasing) as Float;
    let width = height * camera.aspect;
    let render_pixel = |x, y| {
        let mvp_x = 2.0 * (x as Float) / width - 1.0;
        let mvp_y = 1.0 - 2.0 * (y as Float) / height;
        let origin = view_matrix.inverse_transform_point(
            &projection_matrix.unproject_point(&Point::new(mvp_x, mvp_y, -1.0)),
        );
        let target = view_matrix.inverse_transform_point(
            &projection_matrix.unproject_point(&Point::new(mvp_x, mvp_y, 1.0)),
        );
        let origin_to_target = target - origin;
        let max_t = origin_to_target.norm();
        let direction = origin_to_target.unscale(max_t);
        let color = march_ray(origin, direction, max_t, scene);
        color
    };
    let bar = indicatif::ProgressBar::new((width * height) as u64);
    bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {msg} {bar:40.cyan/blue} [ETA: {eta}]")
            .progress_chars("##-"),
    );
    bar.set_message("Rendering");
    bar.set_draw_delta((width * height / 100.0) as u64);
    let image = image::ImageRgb8(image::ImageBuffer::from_fn(
        width as Integer,
        height as Integer,
        |x, y| {
            let pixel = image::Rgb(render_pixel(x, y).to_pixel());
            bar.inc(1);
            pixel
        },
    ));
    bar.set_message("Saving");
    bar.enable_steady_tick(13);
    image
        .resize(
            (width / settings.anti_aliasing as Float) as Integer,
            (height / settings.anti_aliasing as Float) as Integer,
            image::FilterType::Gaussian,
        )
        .save("render.png")
        .unwrap();
    bar.finish();
}

fn march_ray(origin: Point, direction: Vector, max_t: Float, scene: &Scene) -> Color {
    let mut t = 0.0;
    while t < max_t {
        let point = origin + t * direction;
        let (mesh, distance) = scene
            .iter()
            .map(|mesh| (mesh, mesh.geometry.distance(&point)))
            .min_by(|(_mesh1, distance1), (_mesh2, distance2)| {
                distance1.partial_cmp(distance2).unwrap()
            })
            .unwrap();
        if distance < 0.1 {
            return mesh.material.color;
        }
        t += distance;
    }
    Color::new(0.2, 0.2, 0.2)
}

fn main() {
    let red_material = Material {
        color: Color::new(1.0, 0.0, 0.0),
    };
    let scene = vec![
        Mesh {
            geometry: Geometry::Sphere {
                position: Point::new(3.0, 2.0, -10.0),
                radius: 3.0,
            },
            material: red_material,
        },
        Mesh {
            geometry: Geometry::Triangle {
                corners: [
                    Point::new(-2.0, 0.0, 0.0),
                    Point::new(-3.0, 0.0, 0.0),
                    Point::new(-2.0, 1.0, 0.0),
                ],
            },
            material: red_material,
        },
    ];
    let camera = Camera {
        eye: Point::new(0.0, 0.0, 10.0),
        target: Point::new(0.0, 0.0, 0.0),
        aspect: 3.0 / 2.0,
        fovy: 3.14 / 4.0,
        znear: 1.0,
        zfar: 1000.0,
    };
    let settings = RendererSettings {
        definition: 200,
        anti_aliasing: 1,
    };
    render(&scene, &camera, &settings)
}
