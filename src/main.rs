// #![warn(missing_docs)]
// #![warn(clippy::missing_docs_in_private_items)]

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

#[derive(Clone, PartialEq, Eq, Default, Debug)]
struct Tags(pub HashSet<String>);

impl From<HashSet<String>> for Tags {
    fn from(el: HashSet<String>) -> Self {
        Self(el)
    }
}

#[derive(Clone, PartialEq, Eq, Default, Debug)]
struct TaggedImage<P: AsRef<Path>> {
    pub img: P,
    pub tags: Tags,
}

impl<P: AsRef<Path>> TaggedImage<P> {
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
