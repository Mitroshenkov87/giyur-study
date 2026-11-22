//! Playlist order: a shuffled bag with no repeats until every track has
//! played, then a fresh shuffle that avoids starting with the last track.

use rand::seq::SliceRandom;
use rand::Rng;
use std::path::PathBuf;
use std::time::Duration;

/// One entry from `assets/music/manifest.json`, resolved to a file path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub path: PathBuf,
    pub duration: Option<Duration>,
}

/// Shuffle bag over a fixed catalog.
#[derive(Debug, Clone)]
pub struct ShuffleBag {
    tracks: Vec<Track>,
    /// Remaining indices in the current cycle (front is next).
    remaining: Vec<usize>,
    last: Option<usize>,
}

impl ShuffleBag {
    pub fn new(tracks: Vec<Track>) -> Self {
        ShuffleBag {
            tracks,
            remaining: Vec::new(),
            last: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    /// Next track, reshuffling when the current cycle is exhausted.
    pub fn next(&mut self, rng: &mut impl Rng) -> Option<&Track> {
        if self.tracks.is_empty() {
            return None;
        }
        if self.remaining.is_empty() {
            self.reshuffle(rng);
        }
        let i = self.remaining.remove(0);
        self.last = Some(i);
        self.tracks.get(i)
    }

    fn reshuffle(&mut self, rng: &mut impl Rng) {
        let mut order: Vec<usize> = (0..self.tracks.len()).collect();
        order.shuffle(rng);
        if let Some(last) = self.last {
            let n = order.len();
            if n > 1 && order[0] == last {
                // Swap the immediate repeat to the back of the new cycle.
                order.swap(0, n - 1);
            }
        }
        self.remaining = order;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn tracks(n: usize) -> Vec<Track> {
        (0..n)
            .map(|i| Track {
                id: format!("t{i}"),
                title: format!("Track {i}"),
                path: PathBuf::from(format!("t{i}.ogg")),
                duration: Some(Duration::from_secs(10)),
            })
            .collect()
    }

    fn ids(bag: &mut ShuffleBag, rng: &mut impl Rng, n: usize) -> Vec<String> {
        (0..n)
            .map(|_| bag.next(rng).expect("non-empty").id.clone())
            .collect()
    }

    #[test]
    fn empty_bag_yields_none() {
        let mut bag = ShuffleBag::new(Vec::new());
        let mut rng = StdRng::seed_from_u64(1);
        assert!(bag.next(&mut rng).is_none());
        assert!(bag.is_empty());
    }

    #[test]
    fn one_track_repeats() {
        let mut bag = ShuffleBag::new(tracks(1));
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(ids(&mut bag, &mut rng, 5), ["t0", "t0", "t0", "t0", "t0"]);
    }

    #[test]
    fn a_cycle_contains_each_track_once() {
        for seed in 0..20 {
            let mut bag = ShuffleBag::new(tracks(7));
            let mut rng = StdRng::seed_from_u64(seed);
            let cycle = ids(&mut bag, &mut rng, 7);
            let mut sorted = cycle.clone();
            sorted.sort();
            assert_eq!(
                sorted,
                ["t0", "t1", "t2", "t3", "t4", "t5", "t6"],
                "seed {seed}, cycle {cycle:?}"
            );
        }
    }

    #[test]
    fn next_cycle_does_not_start_with_previous_last() {
        for seed in 0..40 {
            let mut bag = ShuffleBag::new(tracks(5));
            let mut rng = StdRng::seed_from_u64(seed);
            let first = ids(&mut bag, &mut rng, 5);
            let second = ids(&mut bag, &mut rng, 5);
            assert_ne!(
                first[4], second[0],
                "seed {seed}: {first:?} then {second:?}"
            );
            let mut sorted = second.clone();
            sorted.sort();
            assert_eq!(sorted, ["t0", "t1", "t2", "t3", "t4"]);
        }
    }
}
