//! Contains stage builders to put in parallel executors when processing images, as well
//! as the definitions of the underlying stages themselves.

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

/* Label constants for different tags, should be moved into a config file eventually */

mod consts {
    #![allow(clippy::missing_docs_in_private_items)]

    pub(super) const CWISE_LABEL: &str = "Rotated 90 degrees clockwise";
    pub(super) const CCWISE_LABEL: &str = "Rotated 90 degrees counterclockwise";
    pub(super) const UPSIDE_DOWN_LABEL: &str = "Upside-down";
    pub(super) const OFF_AXIS_LABEL: &str = "Rotated off-axis";
    pub(super) const BRIGHTEN_LABEL: &str = "Bright";
    pub(super) const DARKEN_LABEL: &str = "Dark";
    pub(super) const BLURRED_LABEL: &str = "Blurred";
}

use consts::*;

/// Converts the radians `rad` to degrees.
fn rad_to_deg(rad: f64) -> f64 {
    rad * 180. / PI
}

/// Converts the degrees `deg` to radians.
fn deg_to_rad(deg: f64) -> f64 {
    deg * PI / 180.
}

/// Creates a builder which will yield `samples` stages, which will rotate the image
/// (without changing the dimensions) between `-deg_limit` and `deg_limit` degrees. It's recommended
/// this value be less than 90, and to combine this stage with `RotationBuilder` for off-axis rotations
/// larger than that. In practice, generally a less extreme value (probably under 30 degrees) is preferable.
pub struct OffAxisRotationBuilder {
    /// The number of variations to build when `build_stage` is called.
    pub samples: usize,
    /// The maximum number of degrees in either direction which a generated stage may rotate an image.
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

/// The actual stage that rotates the image, upon `execute` it will return a new image
/// rotated about the center by `radians` degrees.
pub struct OffAxisStage {
    /// The number of radians to rotate by.
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
            Tags(HashSet::from_iter([OFF_AXIS_LABEL.to_owned()])),
        )
    }

    fn name(&self) -> Cow<str> {
        format!("rot_{:.2}_deg", rad_to_deg(self.radians)).into()
    }
}

/// Not to be confused with `OffAxisRotationBuilder`, this "rotates" the image
/// as if you were to change its exif orientation data - that is to say it simply will
/// create three stages that rotate the image by multiples of 90, 180, and 270 degrees.
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

/// A stage that rotates an image 90 degrees clockwise.
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

/// A stage that rotates an image 90 degrees counterclockwise.
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

/// A stage that flips an image upside down.
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

/// A builder that will yield two stages: a brighten and darken stage, which will change the image
/// pixel intensity across all channels by a random value between `min_luma` and `max_luma`. Note that
/// `i32` is significantly higher than the 8-bit channel value, so this range should be fairly small or
/// all pixels will end up becoming black/white.
pub struct LuminosityBuilder {
    /// The minimum degree of intensity we can brighten/darken by.
    pub min_luma: i32,
    /// The maximum degree of intensity we can brighten/daren by.
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

/// The actual stage that alters brightness and darkness in an image. It will shift all pixels
/// by a constant `value`, negative for darkening and positive for brightening.
pub struct LuminosityStage {
    /// The number to add to all pixel channels in the image.
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

/// A builder that will create `samples` stages that will perform a gaussian blur on the image
/// with a standard deviation between `min_sigma` and `max_sigma` (this is esssentially a uniform
/// distribution over a normal distribution of blurred versions of the image).
pub struct BlurBuilder {
    /// The number of blurred variants to create
    pub samples: usize,
    /// The minimum standard deviation in the gaussian blur kernel
    pub min_sigma: f32,
    /// The maximum standard deviation in the gaussian blur kernel
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

/// The actual stage which blurs the image, it will blur the input image with a gaussian blur
/// whose kernel's standard deviation is `sigma`.
pub struct BlurStage {
    /// The standard deviation of the gaussian blur kernel.
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
