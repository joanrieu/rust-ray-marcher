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
        center: Point,
        radius: Float,
    },
    Triangle {
        axes: [Vector; 3],
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
    base_color: Color,
    diffuse: Float,
    specular: Float,
    shininess: Float,
    is_light: bool,
}

type Color = Vector;

struct Camera {
    eye: Point,
    target: Point,
    up: UnitVector,
    aspect: Float,
    fovy: Float,
    z_near: Float,
    z_far: Float,
}

type Point = nalgebra::Point3<Float>;

type UnitVector = nalgebra::Unit<Vector>;

struct RendererSettings {
    definition: Integer,
    anti_aliasing: Integer,
    epsilon: Float,
    ambient_light: Float,
    // max_bounces: Integer,
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
            axes: [x_axis, y_axis, z_axis],
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

    fn rectangle(vertices: [Point; 4]) -> Self {
        Geometry::Group {
            geometry: vec![
                Geometry::triangle([vertices[0], vertices[1], vertices[2]]),
                Geometry::triangle([vertices[2], vertices[3], vertices[0]]),
            ],
        }
    }

    fn distance(&self, point: &Point, settings: &RendererSettings) -> (&Geometry, Float) {
        match self {
            Geometry::Sphere { center, radius } => {
                (self, nalgebra::distance(point, center) - radius)
            }
            Geometry::Triangle {
                axes: _,
                bounding_sphere_center,
                bounding_sphere_radius,
                transform_01,
                transform_10,
            } => {
                let bounding_sphere_distance =
                    nalgebra::distance(point, bounding_sphere_center) - *bounding_sphere_radius;
                let triangle_depth = 1e-2;
                if bounding_sphere_distance > triangle_depth {
                    (self, bounding_sphere_distance)
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
                    (self, nalgebra::distance(point_0, &projected_0))
                }
            }
            Geometry::Group { geometry } => geometry
                .iter()
                .map(|geometry| geometry.distance(point, settings))
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .unwrap(),
        }
    }
}

fn render(scene: &Scene, camera: &Camera, settings: &RendererSettings, filename: &String) {
    let projection_matrix =
        nalgebra::Perspective3::new(camera.aspect, camera.fovy, camera.z_near, camera.z_far);
    let view_matrix = nalgebra::Isometry3::look_at_rh(&camera.eye, &camera.target, &camera.up);
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
        let (direction, max_distance) = UnitVector::new_and_get(target - origin);
        march_ray(origin, direction, max_distance, scene, settings)
    };
    let bar = indicatif::ProgressBar::new((width * height) as u64);
    bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {msg:10} {bar:40.cyan/blue} [ETA: {eta}]")
            .progress_chars("##-"),
    );
    bar.enable_steady_tick(400);
    bar.set_message("Rendering");
    bar.set_draw_delta((width * height / 100.0) as u64);
    let image = image::ImageRgb8(image::ImageBuffer::from_fn(
        width as Integer,
        height as Integer,
        |x, y| {
            let color = render_pixel(x, y);
            bar.inc(1);
            *color
                .map(|color| {
                    image::Rgb([
                        (color.x.min(1.0) * 255.0) as u8,
                        (color.y.min(1.0) * 255.0) as u8,
                        (color.z.min(1.0) * 255.0) as u8,
                    ])
                })
                .get_or_insert(image::Rgb([0, 0, 0]))
        },
    ));
    bar.set_message("Saving");
    image
        .resize(
            (width / settings.anti_aliasing as Float) as Integer,
            (height / settings.anti_aliasing as Float) as Integer,
            image::FilterType::Gaussian,
        )
        .save(filename)
        .unwrap();
    bar.finish();
}

fn march_ray(
    origin: Point,
    direction: UnitVector,
    max_distance: Float,
    scene: &Scene,
    settings: &RendererSettings,
    // bounces: Integer,
) -> Option<Color> {
    // if bounces < settings.max_bounces {
    let direction = direction.into_inner();
    let mut t = 10.0 * settings.epsilon;
    while t < max_distance {
        let point = origin + t * direction;
        let (mesh, (geometry, distance)) = scene
            .iter()
            // .filter(|mesh| !mesh.material.is_light)
            .map(|mesh| (mesh, mesh.geometry.distance(&point, settings)))
            .min_by(
                |(_mesh_1, (_geometry_1, distance_1)), (_mesh_2, (_geometry_2, distance_2))| {
                    distance_1.partial_cmp(distance_2).unwrap()
                },
            )
            .unwrap();
        if distance < settings.epsilon {
            if mesh.material.is_light {
                return Some(mesh.material.base_color);
            }
            // all vectors start at the contact point
            let view = -direction;
            let normal_signed = normal(geometry, &point).into_inner();
            let normal = normal_signed.dot(&view).signum() * normal_signed;
            // let reflected = incident - 2.0 * normal.dot(&incident) * normal;
            // let reflected_color = march_ray(
            //     point,
            //     UnitVector::new_normalize(reflected),
            //     max_distance,
            //     scene,
            //     settings,
            //     bounces + 1,
            // );
            // let reflection = normal.dot(&incident).abs();
            // return reflected_color.scale(reflection);
            return Some(
                scene
                    .iter()
                    .filter(|mesh| mesh.material.is_light)
                    .map(|light_mesh| {
                        let light = match light_mesh.geometry {
                            Geometry::Sphere { center, radius: _ } => center - point,
                            _ => unimplemented!("lights can only be spheres"),
                        }
                        .normalize();
                        let k_diffuse = mesh.material.diffuse * light.dot(&normal).max(0.0);
                        let reflected = 2.0 * normal.dot(&light) * normal - light;
                        let k_specular = mesh.material.specular
                            * reflected.dot(&view).max(0.0).powf(mesh.material.shininess);
                        // if k > 0.0 {
                        //     println!("l.n = {} r.v = {} k = {}", l_dot_n, r_dot_v, k);
                        // }
                        let diffuse_color = Color::new(
                            light_mesh
                                .material
                                .base_color
                                .x
                                .min(mesh.material.base_color.x),
                            light_mesh
                                .material
                                .base_color
                                .y
                                .min(mesh.material.base_color.y),
                            light_mesh
                                .material
                                .base_color
                                .z
                                .min(mesh.material.base_color.z),
                        );
                        settings.ambient_light * mesh.material.base_color
                            + k_diffuse * diffuse_color
                            + k_specular * light_mesh.material.base_color
                    })
                    .sum(),
            );
        }
        t += distance;
    }
    // }
    None
}

fn normal(geometry: &Geometry, point: &Point) -> UnitVector {
    match geometry {
        Geometry::Sphere { center, radius: _ } => UnitVector::new_normalize(point - center),
        Geometry::Triangle {
            axes,
            bounding_sphere_center: _,
            bounding_sphere_radius: _,
            transform_01: _,
            transform_10: _,
        } => UnitVector::new_normalize(axes[2]),
        Geometry::Group { geometry: _ } => unimplemented!(),
    }
}

fn main() {
    let frames = 40;
    for frame in 0..frames {
        let red_material = Material {
            base_color: Color::new(1.0, 0.0, 0.0),
            specular: 1.0,
            diffuse: 1.0,
            shininess: 10.0,
            is_light: false,
        };
        let light_material = Material {
            base_color: Color::new(1.0, 1.0, 1.0),
            specular: 0.0,
            diffuse: 0.0,
            shininess: 0.0,
            is_light: true,
        };
        let angle = (frame as Float / frames as Float) * 2.0 * std::f32::consts::PI;
        let up = nalgebra::Unit::new_normalize(Vector::new(0.0, 0.0, 1.0));
        let rotation = nalgebra::Isometry3::rotation(up.into_inner() * angle);
        let light = rotation * Point::new(10.0, 2.0, -10.0);
        let scene = vec![
            Mesh {
                geometry: Geometry::Sphere {
                    center: light,
                    radius: 1.0,
                },
                material: light_material,
            },
            Mesh {
                geometry: load_obj("teapot-low.obj"),
                material: red_material,
            },
        ];
        let camera = Camera {
            eye: Point::new(10.0, 50.0, 0.0),
            target: Point::new(0.0, 0.0, 0.0),
            up,
            aspect: 3.0 / 2.0,
            fovy: 3.14 / 4.0,
            z_near: 1.0,
            z_far: 100.0,
        };
        let settings = RendererSettings {
            // definition: 400,
            definition: 40,
            anti_aliasing: 1,
            epsilon: 1e-3,
            ambient_light: 0.2,
            // max_bounces: 3,
        };
        render(
            &scene,
            &camera,
            &settings,
            &format!("render-{:03}.png", frame),
        );
    }
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
                    match points.len() {
                        3 => faces.push(Geometry::triangle([
                            **points.get(0).unwrap(),
                            **points.get(1).unwrap(),
                            **points.get(2).unwrap(),
                        ])),
                        4 => faces.push(Geometry::rectangle([
                            **points.get(0).unwrap(),
                            **points.get(1).unwrap(),
                            **points.get(2).unwrap(),
                            **points.get(3).unwrap(),
                        ])),
                        _ => panic!("unexpected vertex count"),
                    }
                }
                _ => (),
            }
        });
    spinner.finish_and_clear();
    Geometry::Group { geometry: faces }
}
