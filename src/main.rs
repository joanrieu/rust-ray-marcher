extern crate image;
extern crate indicatif;
extern crate nalgebra;

use std::iter::FromIterator;

type Scene = Vec<Mesh>;

struct Mesh {
    geometry: Geometry,
    material: Material,
}

type Float = f32;

type Vector = nalgebra::Vector3<Float>;

enum Geometry {
    Sphere {
        position: Point,
        radius: Float,
    },
    Triangle {
        bounding_sphere_center: Point,
        bounding_sphere_radius: Float,
        transform_01: Matrix,
        transform_10: Matrix,
    },
    Group {
        geometry: Vec<Geometry>,
    },
}

type Matrix = nalgebra::Matrix4<Float>;

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
    z_near: Float,
    z_far: Float,
}

type Point = nalgebra::Point3<Float>;

struct RendererSettings {
    definition: Integer,
    anti_aliasing: Integer,
    epsilon: Float,
    ambient_color: Color,
}

type Integer = u32;

impl Geometry {
    fn triangle(vertices: [Point; 3]) -> Self {
        let bounding_sphere_center = Point::from(
            (vertices[0].coords + vertices[1].coords + vertices[2].coords).unscale(3.0),
        );
        let bounding_sphere_radius = vertices
            .iter()
            .map(|vertex| nalgebra::distance(vertex, &bounding_sphere_center))
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let x_axis = vertices[1] - vertices[0];
        let y_axis = vertices[2] - vertices[0];
        let z_axis = x_axis.cross(&y_axis);
        let base_change_10 = nalgebra::Matrix3::from_columns(&[x_axis, y_axis, z_axis]);
        let translation_10 = nalgebra::Translation3::from(vertices[0].coords);
        let transform_10 = translation_10.to_homogeneous() * base_change_10.to_homogeneous();
        let transform_01 = transform_10.pseudo_inverse(0.0).unwrap();
        Geometry::Triangle {
            bounding_sphere_center,
            bounding_sphere_radius,
            transform_01,
            transform_10,
        }
    }

    fn triangle_strip(vertices: Vec<Point>) -> Self {
        assert!(vertices.len() >= 3);
        let a = &vertices;
        let b = vertices.split_at(1).1;
        let c = vertices.split_at(2).1;
        Geometry::Group {
            geometry: Vec::from_iter(
                a.iter()
                    .zip(b.iter())
                    .zip(c.iter())
                    .map(|((a, b), c)| Self::triangle([*a, *b, *c])),
            ),
        }
    }

    fn distance(&self, point: &Point, settings: &RendererSettings) -> Float {
        match self {
            Geometry::Sphere { position, radius } => nalgebra::distance(point, position) - radius,
            Geometry::Triangle {
                bounding_sphere_center,
                bounding_sphere_radius,
                transform_01,
                transform_10,
            } => {
                let bounding_sphere_distance =
                    nalgebra::distance(point, bounding_sphere_center) - *bounding_sphere_radius;
                let triangle_depth = 0.1;
                if bounding_sphere_distance > triangle_depth {
                    bounding_sphere_distance
                } else {
                    let point_0 = point;
                    let point_1 =
                        Point::from_homogeneous(transform_01 * point_0.to_homogeneous()).unwrap();
                    let x = point_1.coords.x.max(0.0);
                    let y = point_1.coords.y.max(0.0);
                    let z = point_1.coords.z.max(0.0).min(triangle_depth);
                    let w = (x + y).max(1.0);
                    let projected_1 = nalgebra::Vector4::from([x, y, z, w]);
                    let projected_0 = Point::from_homogeneous(transform_10 * projected_1).unwrap();
                    nalgebra::distance(point_0, &projected_0)
                }
            }
            Geometry::Group { geometry } => geometry
                .iter()
                .map(|geometry| geometry.distance(point, settings))
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap(),
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
        nalgebra::Perspective3::new(camera.aspect, camera.fovy, camera.z_near, camera.z_far);
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
        let color = march_ray(origin, direction, max_t, scene, settings);
        color
    };
    let bar = indicatif::ProgressBar::new((width * height) as u64);
    bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {msg} {bar:40.cyan/blue} [ETA: {eta}]")
            .progress_chars("##-"),
    );
    bar.enable_steady_tick(400);
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

fn march_ray(
    origin: Point,
    direction: Vector,
    max_t: Float,
    scene: &Scene,
    settings: &RendererSettings,
) -> Color {
    let mut t = 0.0;
    while t < max_t {
        let point = origin + t * direction;
        let (mesh, distance) = scene
            .iter()
            .map(|mesh| (mesh, mesh.geometry.distance(&point, settings)))
            .min_by(|(_mesh1, distance1), (_mesh2, distance2)| {
                distance1.partial_cmp(distance2).unwrap()
            })
            .unwrap();
        if distance < settings.epsilon {
            return mesh.material.color;
        }
        t += distance;
    }
    settings.ambient_color
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
            geometry: Geometry::triangle_strip(vec![
                Point::new(-2.0, 1.0, 0.0),
                Point::new(-3.0, 0.0, 0.0),
                Point::new(-2.0, 0.0, 0.0),
                Point::new(-1.0, -3.0, 1.0),
            ]),
            material: red_material,
        },
        // Mesh {
        //     geometry: load_obj("teapot-low.obj"),
        //     material: red_material,
        // },
    ];
    let camera = Camera {
        eye: Point::new(0.0, 0.0, 10.0),
        target: Point::new(0.0, 0.0, 0.0),
        aspect: 3.0 / 2.0,
        fovy: 3.14 / 4.0,
        z_near: 1.0,
        z_far: 100.0,
    };
    let settings = RendererSettings {
        definition: 400,
        anti_aliasing: 2,
        epsilon: 0.001,
        ambient_color: Color::new(0.2, 0.2, 0.2),
    };
    render(&scene, &camera, &settings)
}

fn load_obj(path: &str) -> Geometry {
    let spinner = indicatif::ProgressBar::new_spinner();
    let mut points = vec![];
    let mut faces = vec![];
    String::from_utf8(std::fs::read(path).expect("cannot open file"))
        .unwrap()
        .split("\n")
        .for_each(|line| {
            spinner.set_message(line);
            let mut parts = line.trim().split_whitespace();
            match parts.next() {
                Some("v") => {
                    let coords = parts
                        .map(|part| part.parse().expect("cannot parse coordinate"))
                        .collect::<Vec<Float>>();
                    assert_eq!(coords.len(), 3);
                    points.push(Point::new(
                        *coords.get(0).unwrap(),
                        *coords.get(1).unwrap(),
                        *coords.get(2).unwrap(),
                    ))
                }
                Some("f") => {
                    let points = parts
                        .map(|part| {
                            part.split("/")
                                .next()
                                .unwrap()
                                .parse::<usize>()
                                .expect("cannot parse vertex index")
                        })
                        .map(|index| points.get(index - 1).expect("cannot find vertex at index"))
                        .collect::<Vec<&Point>>();
                    assert!(points.len() == 3 || points.len() == 4);
                    if points.len() == 3 {
                        faces.push(Geometry::triangle([
                            **points.get(0).unwrap(),
                            **points.get(1).unwrap(),
                            **points.get(2).unwrap(),
                        ]))
                    } else {
                        faces.push(Geometry::triangle_strip(vec![
                            **points.get(0).unwrap(),
                            **points.get(1).unwrap(),
                            **points.get(2).unwrap(),
                            **points.get(3).unwrap(),
                        ]))
                    }
                }
                _ => (),
            }
        });
    spinner.finish_and_clear();
    Geometry::Group { geometry: faces }
}
