use std::path::{Path, PathBuf};
use std::time::Instant;

use image::imageops::FilterType;
use image::{DynamicImage, ImageReader, RgbImage};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SamplingError {
    #[error("failed to open image: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to decode image: {0}")]
    Decode(#[from] image::ImageError),
}

pub type Result<T> = std::result::Result<T, SamplingError>;

#[derive(Debug, Clone)]
pub struct SampleParams {
    pub path: PathBuf,
    pub stride: u32,
    pub min_lum: u8,
    pub max_samples: usize,
    pub max_dimension: Option<u32>,
    pub seed: u64,
}

impl SampleParams {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            stride: 4,
            min_lum: 0,
            max_samples: 300_000,
            max_dimension: Some(3200),
            seed: 1,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SampleResult {
    pub samples: Vec<[u8; 3]>,
    pub samples_lab: Option<Vec<[f32; 3]>>,
    pub width: u32,
    pub height: u32,
    pub total_pixels: u64,
    pub sampled_pixels: usize,
    pub duration_ms: u128,
}

const LUMA_R: f32 = 0.2126;
const LUMA_G: f32 = 0.7152;
const LUMA_B: f32 = 0.0722;

pub fn prepare_samples(params: &SampleParams) -> Result<SampleResult> {
    let start = Instant::now();

    // Use with_guessed_format() to handle files without extensions
    // This reads the file header to detect the format automatically
    let img = ImageReader::open(&params.path)?
        .with_guessed_format()?
        .decode()?;

    let rgb = to_rgb_with_downscale(img, params.max_dimension);
    let (width, height) = rgb.dimensions();
    let samples = sample_pixels(&rgb, params);
    let samples_lab = samples
        .iter()
        .map(|rgb| crate::color::rgb8_to_lab(*rgb))
        .collect::<Vec<_>>();
    let sampled_pixels = samples.len();

    Ok(SampleResult {
        samples,
        samples_lab: Some(samples_lab),
        width,
        height,
        total_pixels: width as u64 * height as u64,
        sampled_pixels,
        duration_ms: start.elapsed().as_millis(),
    })
}

fn to_rgb_with_downscale(img: DynamicImage, limit: Option<u32>) -> RgbImage {
    let rgb = img.to_rgb8();
    if let Some(max_dim) = limit {
        let (w, h) = rgb.dimensions();
        let current_max = w.max(h);
        if current_max > max_dim {
            let scale = max_dim as f32 / current_max as f32;
            let dst_w = ((w as f32) * scale).round().max(1.0) as u32;
            let dst_h = ((h as f32) * scale).round().max(1.0) as u32;
            return image::imageops::resize(&rgb, dst_w, dst_h, FilterType::Lanczos3);
        }
    }
    rgb
}

fn sample_pixels(img: &RgbImage, params: &SampleParams) -> Vec<[u8; 3]> {
    let stride = params.stride.max(1) as usize;
    let min_lum = params.min_lum as f32;
    let max_samples = if params.max_samples == 0 {
        usize::MAX
    } else {
        params.max_samples
    };

    let (width, height) = img.dimensions();
    let mut samples: Vec<[u8; 3]> =
        Vec::with_capacity(max_samples.min((width as usize) * (height as usize)));

    let mut rng = SmallRng::seed_from_u64(params.seed);
    let mut seen = 0_usize;

    for y in (0..height as usize).step_by(stride) {
        for x in (0..width as usize).step_by(stride) {
            let pixel = img.get_pixel(x as u32, y as u32);
            let [r, g, b] = pixel.0;
            let lum = LUMA_R * (r as f32) + LUMA_G * (g as f32) + LUMA_B * (b as f32);
            if lum < min_lum {
                continue;
            }
            seen += 1;
            if samples.len() < max_samples {
                samples.push([r, g, b]);
            } else {
                let idx = rng.gen_range(0..seen);
                if idx < max_samples {
                    samples[idx] = [r, g, b];
                }
            }
        }
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use tempfile::{Builder, NamedTempFile};

    fn write_temp_image(img: &RgbImage) -> NamedTempFile {
        let file = Builder::new().suffix(".png").tempfile().expect("temp file");
        img.save(file.path()).expect("save image");
        file
    }

    #[test]
    fn sampling_respects_stride_and_luma() {
        let mut img = RgbImage::new(10, 10);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let val = ((x + y) as u8).saturating_mul(20);
            *pixel = Rgb([val, val, val]);
        }
        let tmp = write_temp_image(&img);

        let params = SampleParams {
            path: tmp.path().into(),
            stride: 2,
            min_lum: 60,
            max_samples: 10_000,
            max_dimension: None,
            seed: 42,
        };

        let result = prepare_samples(&params).expect("sample");
        assert!(result.sampled_pixels > 0);
        assert!(result.sampled_pixels <= 25);
        for rgb in result.samples {
            let lum =
                LUMA_R * (rgb[0] as f32) + LUMA_G * (rgb[1] as f32) + LUMA_B * (rgb[2] as f32);
            assert!(lum >= params.min_lum as f32);
        }
    }

    #[test]
    fn reservoir_caps_sample_count() {
        let mut img = RgbImage::new(64, 64);
        for pixel in img.pixels_mut() {
            *pixel = Rgb([10, 20, 30]);
        }
        let tmp = write_temp_image(&img);
        let params = SampleParams {
            path: tmp.path().into(),
            stride: 1,
            min_lum: 0,
            max_samples: 50,
            max_dimension: None,
            seed: 7,
        };
        let result = prepare_samples(&params).expect("sample");
        assert_eq!(result.sampled_pixels, 50);
    }

    #[test]
    fn downscale_limits_dimensions() {
        let mut img = RgbImage::new(4000, 1000);
        for pixel in img.pixels_mut() {
            *pixel = Rgb([120, 80, 40]);
        }
        let tmp = write_temp_image(&img);
        let params = SampleParams {
            path: tmp.path().into(),
            stride: 4,
            min_lum: 0,
            max_samples: 10_000,
            max_dimension: Some(1024),
            seed: 1,
        };
        let result = prepare_samples(&params).expect("sample");
        assert!(result.width <= 1024 && result.height <= 1024);
    }
}
