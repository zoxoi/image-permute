use std::f64::consts::PI;
use std::iter::FromIterator;
use std::{borrow::Cow, collections::HashSet};

use conv::ValueInto;
use image::imageops::colorops;
use image::{imageops, Pixel};
use imageproc::{
    definitions::{Clamp, Image},
    geometric_transformations,
    geometric_transformations::Interpolation,
};
use rand::distributions::Uniform;
use rand::Rng;

use crate::traits::{ImageStage, StageBuilder};
use crate::Tags;

const CWISE_LABEL: &str = "Rotated 90 degrees clockwise";
const CCWISE_LABEL: &str = "Rotated 90 degrees counterclockwise";
const UPSIDE_DOWN_LABEL: &str = "Upside-down";
const OFF_AXIS_LABEL: &str = "Rotated off-axis";
const BRIGHTEN_LABEL: &str = "Bright";
const DARKEN_LABEL: &str = "Dark";
const BLURRED_LABEL: &str = "Blurred";

fn rad_to_deg(rad: f64) -> f64 {
    rad * 180. / PI
}

fn deg_to_rad(deg: f64) -> f64 {
    deg * PI / 180.
}

pub struct OffAxisRotationBuilder {
    pub samples: usize,
    pub deg_limit: f64,
}

impl<P, R> StageBuilder<P, R> for OffAxisRotationBuilder
where
    P: Pixel + Send + Sync + 'static,
    <P as Pixel>::Subpixel: Default + Send + Sync + ValueInto<f32> + Clamp<f32>,
    R: Rng,
{
    fn should_execute(&self, tags: &Tags) -> bool {
        !tags.0.contains(OFF_AXIS_LABEL)
    }

    fn variations(&self) -> usize {
        self.samples
    }

    fn build_stage(&self, rng: &mut R) -> Vec<Box<dyn ImageStage<P> + Send + Sync>> {
        let rad_limit = deg_to_rad(self.deg_limit);
        let range = (-rad_limit)..rad_limit;

        rng.sample_iter(Uniform::from(range))
            .take(self.samples)
            .map(|radians| {
                Box::new(OffAxisStage { radians }) as Box<dyn ImageStage<_> + Send + Sync>
            })
            .collect()
    }
}

pub struct OffAxisStage {
    radians: f64,
}

impl<P> ImageStage<P> for OffAxisStage
where
    P: Pixel + Send + Sync + 'static,
    <P as Pixel>::Subpixel: Default + Send + Sync + ValueInto<f32> + Clamp<f32>,
{
    fn execute(&self, img: &Image<P>) -> (Image<P>, Tags) {
        (
            geometric_transformations::rotate_about_center(
                img,
                self.radians as f32,
                Interpolation::Bicubic,
                P::from_slice(&[Default::default(); 4]).to_owned(),
            ),
            Tags(HashSet::from_iter([CWISE_LABEL.to_owned()])),
        )
    }

    fn name(&self) -> Cow<str> {
        format!("rot_{:.2}_deg", rad_to_deg(self.radians)).into()
    }
}

pub struct RotationBuilder;

impl<P: Pixel + 'static, R: Rng> StageBuilder<P, R> for RotationBuilder {
    fn should_execute(&self, tags: &Tags) -> bool {
        !(tags.0.contains(CWISE_LABEL)
            || tags.0.contains(CCWISE_LABEL)
            || tags.0.contains(UPSIDE_DOWN_LABEL))
    }

    fn variations(&self) -> usize {
        3
    }

    fn build_stage(&self, _: &mut R) -> Vec<Box<dyn ImageStage<P> + Send + Sync>> {
        vec![
            Box::new(ClockwiseStage),
            Box::new(CclockwiseStage),
            Box::new(UpsideDownStage),
        ]
    }
}

pub struct ClockwiseStage;

impl<P: Pixel + 'static> ImageStage<P> for ClockwiseStage {
    fn execute(&self, img: &Image<P>) -> (Image<P>, Tags) {
        (
            imageops::rotate90(img),
            Tags(HashSet::from_iter([CWISE_LABEL.to_owned()])),
        )
    }

    fn name(&self) -> Cow<str> {
        "clowise".into()
    }
}

pub struct CclockwiseStage;

impl<P: Pixel + 'static> ImageStage<P> for CclockwiseStage {
    fn execute(&self, img: &Image<P>) -> (Image<P>, Tags) {
        (
            imageops::rotate270(img),
            Tags(HashSet::from_iter([CCWISE_LABEL.to_owned()])),
        )
    }

    fn name(&self) -> Cow<str> {
        "couwise".into()
    }
}

pub struct UpsideDownStage;

impl<P: Pixel + 'static> ImageStage<P> for UpsideDownStage {
    fn execute(&self, img: &Image<P>) -> (Image<P>, Tags) {
        (
            imageops::rotate180(img),
            Tags(HashSet::from_iter([UPSIDE_DOWN_LABEL.to_owned()])),
        )
    }

    fn name(&self) -> Cow<str> {
        "up_down".into()
    }
}

pub struct LuminosityBuilder {
    pub min_luma: i32,
    pub max_luma: i32,
}

impl<P: Pixel + 'static, R: Rng> StageBuilder<P, R> for LuminosityBuilder {
    fn variations(&self) -> usize {
        2
    }

    fn should_execute(&self, tags: &Tags) -> bool {
        !(tags.0.contains(BRIGHTEN_LABEL) || tags.0.contains(DARKEN_LABEL))
    }

    fn build_stage(&self, rng: &mut R) -> Vec<Box<dyn ImageStage<P> + Send + Sync>> {
        vec![
            Box::new(LuminosityStage {
                value: rng.gen_range(self.min_luma..self.max_luma),
            }),
            Box::new(LuminosityStage {
                value: rng.gen_range(-self.max_luma..-self.min_luma),
            }),
        ]
    }
}

pub struct LuminosityStage {
    value: i32,
}

impl<P: Pixel + 'static> ImageStage<P> for LuminosityStage {
    fn execute(&self, img: &Image<P>) -> (Image<P>, Tags) {
        let mut img = img.clone();
        colorops::brighten_in_place(&mut img, self.value);
        (
            img,
            Tags(HashSet::from_iter([if self.value < 0 {
                DARKEN_LABEL.to_owned()
            } else {
                BRIGHTEN_LABEL.to_owned()
            }])),
        )
    }

    fn name(&self) -> Cow<str> {
        if self.value < 0 {
            format!("dark_{}", self.value).into()
        } else {
            format!("bright_{}", self.value).into()
        }
    }
}

pub struct BlurBuilder {
    pub samples: usize,
    pub min_sigma: f32,
    pub max_sigma: f32,
}

impl<P: Pixel + 'static, R: Rng> StageBuilder<P, R> for BlurBuilder {
    fn variations(&self) -> usize {
        self.samples
    }

    fn should_execute(&self, tags: &Tags) -> bool {
        !(tags.0.contains(BLURRED_LABEL))
    }

    fn build_stage(&self, rng: &mut R) -> Vec<Box<dyn ImageStage<P> + Send + Sync>> {
        rng.sample_iter(Uniform::from(self.min_sigma..self.max_sigma))
            .take(self.samples)
            .map(|sigma| Box::new(BlurStage { sigma }) as Box<dyn ImageStage<_> + Send + Sync>)
            .collect()
    }
}

pub struct BlurStage {
    pub sigma: f32,
}

impl<P: Pixel + 'static> ImageStage<P> for BlurStage {
    fn execute(&self, img: &Image<P>) -> (Image<P>, Tags) {
        (
            imageops::blur(img, self.sigma),
            Tags(HashSet::from_iter([BLURRED_LABEL.to_owned()])),
        )
    }

    fn name(&self) -> Cow<str> {
        format!("blur_{:0.2}", self.sigma).into()
    }
}
