//! This module contains executors for running image processing stages in parallel.

use rayon::prelude::*;
use std::path::Path;

use image::{imageops, Rgba};
use imageproc::definitions::Image;
use rand::{Rng, SeedableRng};

use crate::{traits::StageBuilder, util::SetEnumerator, TaggedImage, Tags};

/// Creates series of stages that can then be [`execute`]d to perform every variation and combination
/// of image transformation requested in parallel.
///
/// [`execute`]: about:blank
pub struct ParallelStageExecutor<R, OP>
where
    R: SeedableRng + Rng,
    OP: AsRef<Path>,
{
    /// A list of builders, that will be executed in order (when present) on each image.
    /// Note that these are *builders* and the stages themselves are built on demand
    /// when given an image during the execution phase.
    ///
    /// The static `rgba8` color space is due to a limitation with `Image` that does not
    /// allow you to convert between color-spaces generically.
    stages: Vec<Box<dyn StageBuilder<Rgba<u8>, R> + Send + Sync>>,

    /// A path to the directory under which to save the output files.
    out_dir: OP,
}

impl<R, OP> ParallelStageExecutor<R, OP>
where
    R: SeedableRng + Rng,
    OP: AsRef<Path> + 'static + Sync,
{
    /// Creates an empty executor (one with no stages), whose output directory
    /// is set to `out_dir`.
    pub fn new(out_dir: OP) -> Self {
        Self {
            stages: vec![],
            out_dir,
        }
    }

    /// Adds a new stage to the executor, for each image all [`StageBuilder::variations()`]
    /// will be generated, including the variations where this stage isn't executed.
    ///
    /// [`StageBuilder::variations()`]: about:blank
    pub(crate) fn add_stage(
        mut self,
        stage: Box<dyn StageBuilder<Rgba<u8>, R> + Send + Sync>,
    ) -> Self {
        self.stages.push(stage);
        self
    }

    /// Executes the pipeline, with a separate worker for each image, each combination/variation
    /// of stages will then be built out for the image, and then those transformations will happen
    /// in parallel. The RNG when building the image will be set based on the image's name.
    pub(crate) fn execute<I, P>(&self, images: I)
    where
        I: IntoParallelIterator<Item = TaggedImage<P>>,
        P: AsRef<Path>,
    {
        images.into_par_iter().for_each(|img| {
            let loaded = match image::open(&img.img) {
                Ok(loaded) => loaded,
                Err(_) => return,
            };
            let name = img.img.as_ref().file_stem().unwrap();
            self.all_pipelines(&img.tags, loaded.to_rgba8(), name.to_str().unwrap())
        });
    }

    /// Executes all pipelines for a single image, this is the workhorse that generates
    /// all stage variations and then schedules them on rayon workers.
    fn all_pipelines(&self, tags: &Tags, img: Image<Rgba<u8>>, name: &str) {
        // TMP, do a better seed fixing
        let seed = name.chars().map(|c| c as u64).sum();

        self.stages
            .iter()
            .map(|bd| bd.variations() * (bd.should_execute(tags) as usize))
            .possibilities()
            .map(|set| {
                set.into_iter()
                    .enumerate()
                    // This generates way more stages than used because we regenerate the variant every time,
                    // however due to the fixed seeding it works out, I do this because Rust would NOT
                    // move the variant out of the vec despite it immediately going out of scope.
                    .filter_map(|(idx, variant)| {
                        let mut rng = R::seed_from_u64(seed);
                        if variant > 0 {
                            // I tried to make this `[variant]` at the end but for some bizarre reason
                            // it won't let me move out of the vector
                            Some((variant, self.stages[idx].build_stage(&mut rng)))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .par_bridge()
            .for_each(|stages| {
                let mut name = name[..name.len().min(10)].to_owned();
                let mut img = img.clone();
                for (variant, stage) in stages {
                    img = stage[variant - 1].execute(&img).0;
                    name = name + "_" + &*stage[variant - 1].name();
                }
                let mut path = self.out_dir.as_ref().to_path_buf();
                path.push(name + ".png");
                imageops::thumbnail(&img, 512, 512).save(path).unwrap();
            });
    }
}
