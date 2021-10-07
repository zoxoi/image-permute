use num::Integer;
use rayon::prelude::*;
use std::{iter::ExactSizeIterator, ops::AddAssign, path::Path};

use image::Rgba;
use imageproc::definitions::Image;
use rand::{Rng, SeedableRng};

use crate::{traits::StageBuilder, TaggedImage, Tags};

pub struct FusedExecutor<R, OP>
where
    R: SeedableRng + Rng,
    OP: AsRef<Path>,
{
    stages: Vec<Box<dyn StageBuilder<Rgba<u8>, R> + Send + Sync>>,
    out_dir: OP,
}

impl<R, OP> FusedExecutor<R, OP>
where
    R: SeedableRng + Rng,
    OP: AsRef<Path> + 'static + Sync,
{
    pub fn new(out_dir: OP) -> Self {
        Self {
            stages: vec![],
            out_dir,
        }
    }

    pub(crate) fn add_stage(
        mut self,
        stage: Box<dyn StageBuilder<Rgba<u8>, R> + Send + Sync>,
    ) -> Self {
        self.stages.push(stage);
        self
    }

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

    fn all_pipelines(&self, tags: &Tags, img: Image<Rgba<u8>>, name: &str) {
        // TMP, do a better seed fixing
        let seed = name.chars().map(|c| c as u64).sum();

        self.stages
            .iter()
            .map(|bd| bd.variations() * (bd.should_execute(tags) as usize))
            .power_set()
            .map(|set| {
                set.into_iter()
                    .enumerate()
                    // This generates way more stages than used because we regenerate the variant every time,
                    // however due to the fixed seeding it works out, I do this because Rust would NOT
                    // move the variant out the vec despite it immediately going out of scope.
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
                let mut name = name.to_owned();
                let mut img = img.clone();
                for (variant, stage) in stages {
                    img = stage[variant - 1].execute(&img).0;
                    name = name + "_" + &*stage[variant - 1].name();
                }
                let mut path = self.out_dir.as_ref().to_path_buf();
                path.push(name + ".png");
                img.save(path).unwrap();
            });
    }
}
pub trait PowerSetAdapter<N>: ExactSizeIterator<Item = N>
where
    N: Integer,
{
    #[inline]
    fn power_set(self) -> PowerSetIterator<N>
    where
        Self: Sized,
    {
        PowerSetIterator {
            maxes: self.collect(),
            variation: None,
            finished: false,
        }
    }
}

impl<N, I> PowerSetAdapter<N> for I
where
    N: Integer,
    I: ExactSizeIterator<Item = N>,
{
}

pub struct PowerSetIterator<N>
where
    N: Integer,
{
    maxes: Vec<N>,
    variation: Option<Vec<N>>,
    finished: bool,
}

impl<'a, N> Iterator for PowerSetIterator<N>
where
    N: Integer + AddAssign + Clone + Copy,
{
    type Item = Vec<N>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        match self.variation {
            None => {
                let variation = vec![N::zero(); self.maxes.len()];
                self.variation = Some(variation.clone());
                Some(variation)
            }
            Some(ref mut variation) => {
                variation[0] += N::one();
                for (idx, max) in self.maxes.iter().enumerate() {
                    if variation[idx] > *max {
                        variation[idx] = N::zero();
                        if idx < variation.len() - 1 {
                            variation[idx + 1] += N::one();
                        } else {
                            self.finished = true;
                            return None;
                        }
                    } else {
                        break;
                    }
                }

                Some(variation.clone())
            }
        }
    }
}
