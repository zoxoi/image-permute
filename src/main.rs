// #![warn(missing_docs)]
// #![warn(clippy::missing_docs_in_private_items)]

//! A utility for parallel image transformations

use glob::glob;
use rand::prelude::*;

mod executors;
mod stages;
mod traits;

// TODO: Stages =

use std::{collections::HashSet, fs, iter::Iterator, path::Path};

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
    use executors::FusedExecutor;
    use stages::{OffAxisRotationBuilder, RotationBuilder};

    let files: Vec<_> = glob("./images/*")
        .unwrap()
        .map(|fname| TaggedImage::from_iter(fname.unwrap(), vec![]))
        .collect();

    let transformer: FusedExecutor<StdRng, _> = FusedExecutor::new("./processed")
        .add_stage(Box::new(OffAxisRotationBuilder {
            samples: 3,
            deg_limit: 80.,
        }))
        .add_stage(Box::new(RotationBuilder));

    fs::remove_dir_all("./processed").unwrap_or(());
    fs::create_dir("./processed").unwrap_or(());

    transformer.execute(files);
}
