//! A scene: camera, geometry, environment, and the light list built from them.

use crate::environment::{luminance, Environment};
use crate::light::{Light, LightKind, LightSample};
use crate::intersectable::Intersectable;
use crate::{Camera, Color, Vec3, BVH};

use std::f64::consts::PI;

#[derive(Debug)]
pub struct Scene {
    pub camera: Camera,
    pub world: BVH,
    pub environment: Environment,
    lights: Vec<Light>,
    /// Cumulative selection probabilities, parallel to `lights`.
    light_cdf: Vec<f64>,
    /// Index into `lights` for each shape, or `usize::MAX`.
    light_of_shape: Vec<usize>,
    sky_light: Option<usize>,
    sun_light: Option<usize>,
}

impl Scene {
    pub fn new(camera: Camera, world: BVH) -> Self {
        let mut scene = Self {
            camera,
            world,
            environment: Environment::default(),
            lights: Vec::new(),
            light_cdf: Vec::new(),
            light_of_shape: Vec::new(),
            sky_light: None,
            sun_light: None,
        };
        scene.build_lights();
        scene
    }

    pub fn with_environment(mut self, environment: Environment) -> Self {
        self.environment = environment;
        self.build_lights();
        self
    }

    /// Collect lights weighted by emitted power. Shapes: π L A. Infinite
    /// lights: their irradiance times the projected area of the scene's
    /// bounding sphere, so they compare sensibly with area lights.
    fn build_lights(&mut self) {
        let mut lights = Vec::new();
        let mut powers = Vec::new();
        let shapes = self.world.shapes();
        self.light_of_shape = vec![usize::MAX; shapes.len()];
        for (shape_id, shape) in shapes.iter().enumerate() {
            if let Some((light_shape, emission)) = shape.as_light() {
                self.light_of_shape[shape_id] = lights.len();
                powers.push((PI * luminance(emission) * light_shape.area()).max(0.0));
                lights.push(Light {
                    kind: LightKind::Shape {
                        shape_id,
                        shape: light_shape,
                    },
                    select_pdf: 0.0,
                });
            }
        }

        let bounds = self.world.bounding_box();
        let radius = (bounds.max - bounds.min).length() * 0.5;
        let projected_area = PI * radius * radius;

        self.sky_light = None;
        self.sun_light = None;
        if let Some(sky) = &self.environment.sky {
            self.sky_light = Some(lights.len());
            powers.push((sky.irradiance() * projected_area).max(0.0));
            lights.push(Light {
                kind: LightKind::Sky,
                select_pdf: 0.0,
            });
        }
        if let Some(sun) = &self.environment.sun {
            self.sun_light = Some(lights.len());
            powers.push((sun.irradiance() * projected_area).max(0.0));
            lights.push(Light {
                kind: LightKind::Sun,
                select_pdf: 0.0,
            });
        }

        let total: f64 = powers.iter().sum();
        let mut cdf = Vec::with_capacity(lights.len());
        let mut cumulative = 0.0;
        for (light, power) in lights.iter_mut().zip(&powers) {
            light.select_pdf = if total > 0.0 {
                power / total
            } else {
                1.0 / powers.len() as f64
            };
            cumulative += light.select_pdf;
            cdf.push(cumulative);
        }

        self.lights = lights;
        self.light_cdf = cdf;
    }

    /// All lights in the scene.
    pub fn lights(&self) -> &[Light] {
        &self.lights
    }

    /// Pick a light with probability proportional to its power, from a
    /// uniform `u` in [0, 1). Returns the light and its selection pdf.
    pub fn pick_light(&self, u: f64) -> (&Light, f64) {
        let index = self
            .light_cdf
            .partition_point(|&cdf| cdf <= u)
            .min(self.lights.len() - 1);
        let light = &self.lights[index];
        (light, light.select_pdf)
    }

    /// The light corresponding to a shape id, if that shape emits.
    pub fn light_of_shape(&self, shape_id: usize) -> Option<&Light> {
        self.lights.get(*self.light_of_shape.get(shape_id)?)
    }

    pub fn sky_light(&self) -> Option<&Light> {
        self.sky_light.map(|i| &self.lights[i])
    }

    pub fn sun_light(&self) -> Option<&Light> {
        self.sun_light.map(|i| &self.lights[i])
    }

    /// Sample a direction from `p` towards `light`.
    pub fn sample_light(&self, light: &Light, p: Vec3, u1: f64, u2: f64) -> Option<LightSample> {
        match light.kind {
            LightKind::Shape { shape, .. } => shape.sample(p, u1, u2),
            LightKind::Sky => {
                let (direction, pdf) = self.environment.sky.as_ref()?.sample(u1, u2);
                Some(LightSample {
                    direction,
                    point: p + direction,
                    pdf,
                })
            }
            LightKind::Sun => {
                let sun = self.environment.sun.as_ref()?;
                Some(LightSample {
                    direction: sun.sample(u1, u2),
                    point: p + sun.direction,
                    pdf: sun.pdf(),
                })
            }
        }
    }

    /// Solid-angle pdf that `sample_light` would produce unit `direction`
    /// from `p`, reaching the light at `point` (shapes only).
    pub fn light_pdf(&self, light: &Light, p: Vec3, point: Vec3, direction: Vec3) -> f64 {
        match light.kind {
            LightKind::Shape { shape, .. } => shape.pdf(p, point, direction),
            LightKind::Sky => self.environment.sky.as_ref().map_or(0.0, |s| s.pdf(direction)),
            LightKind::Sun => self
                .environment
                .sun
                .as_ref()
                .map_or(0.0, |s| if s.contains(direction) { s.pdf() } else { 0.0 }),
        }
    }

    /// Radiance seen by a ray that leaves the scene in unit `direction`.
    pub fn environment_radiance(&self, direction: Vec3) -> Color {
        let mut c = Color::new(0.0, 0.0, 0.0);
        if let Some(sky) = &self.environment.sky {
            c += sky.radiance(direction);
        }
        if let Some(sun) = &self.environment.sun {
            if sun.contains(direction) {
                c += sun.radiance;
            }
        }
        c
    }
}
