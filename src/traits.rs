use std::borrow::Cow;

use crate::Tags;
use image::Pixel;
use imageproc::definitions::Image;
use rand::Rng;

pub(crate) trait StageBuilder<P: Pixel, R: Rng> {
    fn should_execute(&self, tags: &Tags) -> bool;

    fn variations(&self) -> usize;

    fn build_stage(&self, rng: &mut R) -> Vec<Box<dyn ImageStage<P> + Send + Sync>>;
}

pub(crate) trait ImageStage<P: Pixel> {
    fn execute(&self, img: &Image<P>) -> (Image<P>, Tags);

    fn name(&self) -> Cow<str>;
}
