//! How far along one file, and one run, actually are.
//!
//! A conversion is not one long ffmpeg call. It can choose a quality setting by
//! test-encoding samples, encode the file, measure the result, validate it and
//! then commit it, and only the encode ever reported progress. This module
//! turns those phases into one number per file and one per run.
//!
//! Three rules keep the number honest:
//!
//! - A phase's denominator is its worst case, so the bar under-reports and then
//!   snaps forward rather than over-reporting and stalling.
//! - Finishing a phase early snaps forward, never backwards.
//! - A retry remaps into the range that is left instead of starting over.

/// The stages one file passes through, in order.
///
/// Validating comes before measuring because that is the order the pipeline
/// runs them in: an output is checked for what it must contain before anyone
/// asks how good it looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Test-encoding samples to find a quality setting.
    Choosing,
    /// The encode itself.
    Encoding,
    /// Checking the result is what was asked for.
    Validating,
    /// Comparing the result against the source.
    Measuring,
    /// Putting the result in place.
    Committing,
}

/// Every phase, in the order a file passes through them.
pub const PHASES: [Phase; 5] = [
    Phase::Choosing,
    Phase::Encoding,
    Phase::Validating,
    Phase::Measuring,
    Phase::Committing,
];

impl Phase {
    // AI-FUNC-SUMMARY: Maps a phase to its stable wire value; returns static label; side effects: none.
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Choosing => "choosing",
            Phase::Encoding => "encoding",
            Phase::Measuring => "measuring",
            Phase::Validating => "validating",
            Phase::Committing => "committing",
        }
    }

    // AI-FUNC-SUMMARY: Maps a phase to the words shown beside a terminal progress bar; returns static label; side effects: none.
    pub fn label(self) -> &'static str {
        match self {
            Phase::Choosing => "choosing quality",
            Phase::Encoding => "encoding",
            Phase::Measuring => "measuring quality",
            Phase::Validating => "validating",
            Phase::Committing => "committing",
        }
    }
}

/// How thoroughly the result will be compared against the source, which is the
/// one phase whose cost changes by more than a rounding error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurePlan {
    /// No comparison is possible, so the phase does not exist.
    None,
    /// Every few frames.
    Sampled,
    /// Every frame, which costs about as much as the encode.
    Full,
}

/// What share of one file each phase accounts for.
///
/// Decided before the file starts, from what the run is configured to do, so
/// no phase can appear part-way through and push the bar backwards.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhaseWeights {
    choosing: f64,
    encoding: f64,
    measuring: f64,
    validating: f64,
    committing: f64,
}

impl PhaseWeights {
    // AI-FUNC-SUMMARY:
    // Purpose: Decides how much of a file each phase accounts for.
    // Inputs: Whether a sample search runs and how the result will be measured.
    // Returns: Weights that sum to one.
    // Side effects: None.
    // Notes: The raw numbers come from the measured costs in doc/en-us/quality.md; only their ratio matters.
    pub fn new(searching: bool, measuring: MeasurePlan) -> Self {
        let raw = Self {
            choosing: if searching { 20.0 } else { 0.0 },
            encoding: 60.0,
            measuring: match measuring {
                MeasurePlan::None => 0.0,
                MeasurePlan::Sampled => 15.0,
                MeasurePlan::Full => 45.0,
            },
            validating: 3.0,
            committing: 2.0,
        };
        let total = raw.choosing + raw.encoding + raw.measuring + raw.validating + raw.committing;
        Self {
            choosing: raw.choosing / total,
            encoding: raw.encoding / total,
            measuring: raw.measuring / total,
            validating: raw.validating / total,
            committing: raw.committing / total,
        }
    }

    // AI-FUNC-SUMMARY: Reports the share of a file one phase accounts for; returns a fraction of one; side effects: none.
    pub fn weight(&self, phase: Phase) -> f64 {
        match phase {
            Phase::Choosing => self.choosing,
            Phase::Encoding => self.encoding,
            Phase::Measuring => self.measuring,
            Phase::Validating => self.validating,
            Phase::Committing => self.committing,
        }
    }

    // AI-FUNC-SUMMARY: Sums the shares of every phase before the given one; returns a fraction of one; side effects: none.
    pub fn completed_before(&self, phase: Phase) -> f64 {
        PHASES
            .iter()
            .take_while(|candidate| **candidate != phase)
            .map(|candidate| self.weight(*candidate))
            .sum()
    }
}

/// Where one file has got to.
#[derive(Debug, Clone)]
pub struct FileProgress {
    weights: PhaseWeights,
    phase: Phase,
    step: usize,
    steps: usize,
    fraction: f64,
    /// What the file had reached when the current attempt began, so a retry
    /// carries on from there instead of starting over.
    floor: f64,
    /// The last percentage reported, which is what stops any arithmetic here
    /// from ever moving the bar backwards.
    reported: f64,
}

impl FileProgress {
    // AI-FUNC-SUMMARY: Starts tracking one file; returns a new tracker positioned at its first phase; side effects: none.
    pub fn new(weights: PhaseWeights) -> Self {
        let phase = first_active_phase(&weights);
        Self {
            weights,
            phase,
            step: 1,
            steps: 1,
            fraction: 0.0,
            floor: 0.0,
            reported: 0.0,
        }
    }

    // AI-FUNC-SUMMARY: Reports which phase the file is in; returns the current phase; side effects: none.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    // AI-FUNC-SUMMARY: Reports which run of the current phase is under way; returns the step and the phase's worst case; side effects: none.
    pub fn step(&self) -> (usize, usize) {
        (self.step, self.steps)
    }

    // AI-FUNC-SUMMARY: Moves the file into a phase and states that phase's worst-case number of runs; returns none; side effects: updates the tracker.
    pub fn enter(&mut self, phase: Phase, steps: usize) {
        self.phase = phase;
        self.steps = steps.max(1);
        self.step = 1;
        self.fraction = 0.0;
    }

    // AI-FUNC-SUMMARY: Marks which run of the current phase is under way; returns none; side effects: updates the tracker.
    pub fn enter_step(&mut self, step: usize) {
        self.step = step.clamp(1, self.steps);
        self.fraction = 0.0;
    }

    // AI-FUNC-SUMMARY: Records how far the run in progress has got; returns none; side effects: updates the tracker.
    pub fn advance(&mut self, fraction: f64) {
        self.fraction = fraction.clamp(0.0, 1.0);
    }

    // AI-FUNC-SUMMARY: Closes the current phase, whether or not its worst case was reached; returns none; side effects: updates the tracker.
    pub fn finish(&mut self, phase: Phase) {
        self.phase = phase;
        self.step = self.steps;
        self.fraction = 1.0;
        self.reported = self.reported.max(self.remap(self.raw_percent()));
        if let Some(next) = next_active_phase(&self.weights, phase) {
            self.enter(next, 1);
        }
    }

    // AI-FUNC-SUMMARY: Skips the file forward to a phase, closing everything before it; returns none; side effects: updates the tracker.
    pub fn finish_before(&mut self, phase: Phase) {
        self.phase = phase;
        self.steps = 1;
        self.step = 1;
        self.fraction = 0.0;
        let boundary = self.remap(self.weights.completed_before(phase) * 100.0);
        self.reported = self.reported.max(boundary);
    }

    // AI-FUNC-SUMMARY: Records that a fresh encode attempt is starting; returns none; side effects: pins the percentage reached so far as a floor.
    pub fn begin_attempt(&mut self) {
        self.floor = self.percent();
        self.reported = self.floor;
        let phase = first_active_phase(&self.weights);
        self.enter(phase, 1);
    }

    // AI-FUNC-SUMMARY: Reports how far through this file the work is; returns a percentage that never decreases; side effects: updates the remembered high-water mark.
    pub fn percent(&self) -> f64 {
        self.remap(self.raw_percent()).clamp(self.reported, 100.0)
    }

    // AI-FUNC-SUMMARY: Places a percentage of this attempt inside the range the file has left; returns a percentage; side effects: none.
    fn remap(&self, raw: f64) -> f64 {
        self.floor + (100.0 - self.floor) * raw / 100.0
    }

    // AI-FUNC-SUMMARY: Reports how far through the current phase the work is; returns a percentage; side effects: none.
    pub fn phase_percent(&self) -> f64 {
        let steps = self.steps.max(1) as f64;
        let step = self.step.max(1) as f64;
        ((step - 1.0 + self.fraction) / steps * 100.0).clamp(0.0, 100.0)
    }

    // AI-FUNC-SUMMARY: Remembers the percentage just reported so later ones cannot be lower; returns none; side effects: updates the tracker.
    pub fn hold(&mut self, percent: f64) {
        self.reported = self.reported.max(percent);
    }

    // AI-FUNC-SUMMARY: Computes the percentage the phase weights imply, before the retry floor is applied; returns a percentage; side effects: none.
    fn raw_percent(&self) -> f64 {
        let completed = self.weights.completed_before(self.phase);
        let within = self.weights.weight(self.phase) * self.phase_percent() / 100.0;
        ((completed + within) * 100.0).clamp(0.0, 100.0)
    }
}

// AI-FUNC-SUMMARY: Finds the first phase a run actually performs; returns that phase; side effects: none.
fn first_active_phase(weights: &PhaseWeights) -> Phase {
    PHASES
        .iter()
        .copied()
        .find(|phase| weights.weight(*phase) > 0.0)
        .unwrap_or(Phase::Encoding)
}

// AI-FUNC-SUMMARY: Finds the next phase a run performs after the given one; returns that phase, or none when the file is finished; side effects: none.
fn next_active_phase(weights: &PhaseWeights, after: Phase) -> Option<Phase> {
    PHASES
        .iter()
        .copied()
        .skip_while(|phase| *phase != after)
        .skip(1)
        .find(|phase| weights.weight(*phase) > 0.0)
}

/// How far a whole run has got.
///
/// The numerator counts every candidate the run has finished with, converted,
/// failed or skipped alike. Counting conversions instead would leave the bar
/// stuck whenever most of a folder is skipped, which is the common case for a
/// second run over the same videos.
#[derive(Debug, Clone, Copy, Default)]
pub struct RunProgress {
    processed: usize,
    total: usize,
}

impl RunProgress {
    // AI-FUNC-SUMMARY: Starts tracking a run over a known number of candidates; returns a new tracker; side effects: none.
    pub fn new(total: usize) -> Self {
        Self {
            processed: 0,
            total,
        }
    }

    // AI-FUNC-SUMMARY: Records that the run is finished with one candidate; returns none; side effects: updates the tracker.
    pub fn finish_candidate(&mut self) {
        self.processed += 1;
    }

    // AI-FUNC-SUMMARY: Reports how many candidates are done and how many there are; returns both counts; side effects: none.
    pub fn counts(&self) -> (usize, usize) {
        (self.processed, self.total)
    }

    // AI-FUNC-SUMMARY: Reports how far through the run the work is, counting the file in progress; returns a percentage; side effects: none.
    pub fn percent(&self, current_file_percent: f64) -> f64 {
        if self.total == 0 {
            return 100.0;
        }
        let done = self.processed as f64 + current_file_percent.clamp(0.0, 100.0) / 100.0;
        (done / self.total as f64 * 100.0).clamp(0.0, 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_weights() -> PhaseWeights {
        PhaseWeights::new(true, MeasurePlan::Sampled)
    }

    #[test]
    fn phase_weights_sum_to_one() {
        let cases = [
            PhaseWeights::new(true, MeasurePlan::Sampled),
            PhaseWeights::new(false, MeasurePlan::Sampled),
            PhaseWeights::new(true, MeasurePlan::Full),
            PhaseWeights::new(false, MeasurePlan::None),
        ];
        for weights in cases {
            let total: f64 = PHASES.iter().map(|phase| weights.weight(*phase)).sum();
            assert!((total - 1.0).abs() < 1e-9, "weights summed to {total}");
        }
    }

    #[test]
    fn the_default_run_spends_most_of_a_file_encoding() {
        let weights = default_weights();
        assert!((weights.weight(Phase::Choosing) - 0.20).abs() < 1e-9);
        assert!((weights.weight(Phase::Encoding) - 0.60).abs() < 1e-9);
        assert!((weights.weight(Phase::Measuring) - 0.15).abs() < 1e-9);
    }

    #[test]
    fn a_skipped_phase_has_no_weight() {
        let weights = PhaseWeights::new(false, MeasurePlan::None);
        assert_eq!(weights.weight(Phase::Choosing), 0.0);
        assert_eq!(weights.weight(Phase::Measuring), 0.0);
        assert!(weights.weight(Phase::Encoding) > 0.9);
    }

    #[test]
    fn a_file_starts_at_the_first_phase_it_performs() {
        let searching = FileProgress::new(default_weights());
        assert_eq!(searching.phase(), Phase::Choosing);
        let estimating = FileProgress::new(PhaseWeights::new(false, MeasurePlan::Sampled));
        assert_eq!(estimating.phase(), Phase::Encoding);
    }

    #[test]
    fn phase_percent_is_monotonic() {
        let mut progress = FileProgress::new(default_weights());
        let mut last = 0.0;
        let script = [
            (Phase::Choosing, 39_usize),
            (Phase::Encoding, 1),
            (Phase::Validating, 1),
            (Phase::Measuring, 2),
            (Phase::Committing, 1),
        ];
        for (phase, steps) in script {
            progress.enter(phase, steps);
            for step in 1..=steps {
                progress.enter_step(step);
                for tenth in 0..=10 {
                    progress.advance(f64::from(tenth) / 10.0);
                    let percent = progress.percent();
                    assert!(percent >= last, "{percent} came after {last}");
                    progress.hold(percent);
                    last = percent;
                }
            }
        }
        assert!(
            (last - 100.0).abs() < 1e-9,
            "a finished file reported {last}"
        );
    }

    #[test]
    fn finishing_a_phase_early_snaps_forward() {
        let mut progress = FileProgress::new(default_weights());
        progress.enter(Phase::Choosing, 39);
        progress.enter_step(2);
        assert!(progress.percent() < 2.0);
        progress.finish(Phase::Choosing);
        assert!((progress.percent() - 20.0).abs() < 1e-9);
        assert_eq!(progress.phase(), Phase::Encoding);
    }

    #[test]
    fn a_retry_never_moves_the_bar_backwards() {
        let mut progress = FileProgress::new(default_weights());
        progress.finish(Phase::Choosing);
        progress.enter(Phase::Encoding, 1);
        progress.advance(0.5);
        let before = progress.percent();
        assert!(before > 45.0 && before < 55.0, "reached {before}");

        progress.begin_attempt();
        assert!((progress.percent() - before).abs() < 1e-9);

        progress.enter(Phase::Encoding, 1);
        progress.advance(1.0);
        assert!(progress.percent() > before);

        progress.finish(Phase::Committing);
        assert!((progress.percent() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn overall_percent_counts_every_candidate() {
        let mut run = RunProgress::new(10);
        assert_eq!(run.percent(0.0), 0.0);
        for _ in 0..7 {
            run.finish_candidate();
        }
        assert!((run.percent(0.0) - 70.0).abs() < 1e-9);
        for _ in 0..3 {
            run.finish_candidate();
        }
        assert!((run.percent(0.0) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn overall_percent_advances_with_the_current_file() {
        let mut run = RunProgress::new(10);
        for _ in 0..4 {
            run.finish_candidate();
        }
        assert!((run.percent(50.0) - 45.0).abs() < 1e-9);
    }

    #[test]
    fn an_empty_run_is_finished() {
        assert_eq!(RunProgress::new(0).percent(0.0), 100.0);
    }
}
