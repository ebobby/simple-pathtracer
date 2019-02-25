use super::camera::Camera;
use super::color::Color;
use super::intersectable::IntersectableList;
use super::ray::Ray;

#[derive(Debug)]
pub struct Scene {
    pub width: u32,
    pub height: u32,
    pub samples: u32,

    pub camera: Camera,
    pub objects: IntersectableList,
}

impl Scene {
    pub fn render(&self, filename: String) {
        let mut imgbuf = image::ImageBuffer::new(self.width, self.height);

        let gamma_correction = (2.2f64).recip();

        let w = f64::from(self.width);
        let h = f64::from(self.height);
        let s = f64::from(self.samples);

        // Iterate over the coordinates and pixels of the image
        for y in 0..self.height {
            for x in 0..self.width {
                let mut color = Color::black();

                for _i in 0..self.samples {
                    let u = (f64::from(x) + rand::random::<f64>()) / w;
                    let v = (f64::from(y) + rand::random::<f64>()) / h;

                    let ray = self.camera.get_ray(u, v);

                    color += self.color(ray, 1);
                }

                color = color / s;

                let pixel = imgbuf.get_pixel_mut(x, y);

                *pixel = color.gamma_rgb(gamma_correction);
            }
        }

        imgbuf.save(filename).unwrap();
    }

    fn color(&self, ray: Ray, depth: u32) -> Color {
        if depth >= 50 {
            return Color::black();
        }

        if let Some(intersection) = self.objects.intersect(ray) {
            if let Some(scattered) = intersection.material.scatter(ray, &intersection) {
                scattered.attenuation * self.color(scattered.scattered, depth + 1)
            } else {
                Color::black()
            }
        } else {
            let unit_direction = ray.direction.normalize();
            let t = (unit_direction.y + 1.0) * 0.5;

            Color::new(1.0, 1.0, 1.0) * (1.0 - t) + Color::new(0.5, 0.7, 1.0) * t
        }
    }
}
