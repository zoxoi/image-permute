#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

//! A utility for parallel image transformations

use glob::glob;
use rand::prelude::*;

mod executors;
mod stages;
mod traits;
mod util;

// TODO: Stages =

use std::{collections::HashSet, fs, iter::Iterator, path::Path};

use crate::stages::BlurBuilder;

/// A newtype over a `HashSet` meant to contain image labels used
/// to determine if a stage should be executed on an image or not.
#[derive(Clone, PartialEq, Eq, Default, Debug)]
struct Tags(pub HashSet<String>);

impl From<HashSet<String>> for Tags {
    fn from(el: HashSet<String>) -> Self {
        Self(el)
    }
}

/// Combines a path to an image on disk with its associated [`Tags`].
///
/// [`Tags`]: about:blank
#[derive(Clone, PartialEq, Eq, Default, Debug)]
struct TaggedImage<P: AsRef<Path>> {
    /// A path to the image that will be manipulated.
    pub img: P,
    /// The associated tags (blurred, darkened, etc) of the image.
    pub tags: Tags,
}

impl<P: AsRef<Path>> TaggedImage<P> {
    /// Creates a new `TaggedImage` for the image at the path `P`, whose tags
    /// are build by `collect`ing the strings in the `tags` iterator into a
    /// `HashSet`.
    fn from_iter<I: IntoIterator<Item = String>>(path: P, tags: I) -> Self {
        Self {
            img: path,
            tags: Tags(tags.into_iter().collect()),
        }
    }
}

fn main() {
    use executors::ParallelStageExecutor;
    use stages::RotationBuilder;

    let files: Vec<_> = glob("./images/*")
        .unwrap()
        .map(|fname| TaggedImage::from_iter(fname.unwrap(), vec![]))
        .collect();

    let transformer: ParallelStageExecutor<StdRng, _> = ParallelStageExecutor::new("./processed")
        .add_stage(Box::new(BlurBuilder {
            samples: 1,
            min_sigma: 5.,
            max_sigma: 10.,
        }))
        .add_stage(Box::new(RotationBuilder));

    fs::remove_dir_all("./processed").unwrap_or(());
    fs::create_dir("./processed").unwrap_or(());

    transformer.execute(files);
}
