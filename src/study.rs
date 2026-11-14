// SPDX-License-Identifier: LGPL-3.0-or-later
//! G7 — study layer.
//!
//! The campaign object is ours: it owns the queue, the pins, the trial ledger,
//! the pruner, and the importance readout. The external `blackopt`/`optimizer`
//! crates stay samplers behind that boundary; this module never lets a sampler
//! become the source of truth for a campaign.
//!
//! The workflow is deliberately explicit:
//!
//! 1. enqueue proposals (GA offspring, QD bright cells, mined motifs, ...);
//! 2. pin parameters that must not move (for example a fixed substrate);
//! 3. ask for the next trial, evaluate it, and record its merit;
//! 4. prune dead tribes, then explain the completed trials.
//!
//! Every value is deterministic and recorded, so two campaigns with the same
//! queue and results produce the same ledger.

use std::collections::{BTreeMap, VecDeque};

/// Where a proposal came from. Kept in the ledger for pruning and credit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProposalSource {
    /// Explicitly enqueued by the caller.
    Enqueued,
    /// Produced by a GA generation.
    Ga,
    /// Produced by the needle/memetic operator.
    Needle,
    /// Produced by a QD archive cell.
    Archive,
    /// Produced by a screening or surrogate stage.
    Screening,
}

/// One queued parameter set.
#[derive(Clone, Debug, PartialEq)]
pub struct Proposal {
    pub id: u64,
    pub params: BTreeMap<String, f64>,
    pub source: ProposalSource,
}

/// Lifecycle of a trial in the ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrialState {
    /// Returned by [`Campaign::next_trial`] and not yet recorded.
    Running,
    /// Recorded with a finite merit.
    Complete,
    /// Excluded by the pruner; it will not be selected again.
    Pruned,
}

/// One recorded trial.
#[derive(Clone, Debug)]
pub struct TrialRecord {
    pub id: u64,
    pub params: BTreeMap<String, f64>,
    pub source: ProposalSource,
    pub state: TrialState,
    /// `None` while running.
    pub merit: Option<f64>,
}

/// Campaign limits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StudyConfig {
    /// Maximum completed trials before the queue refuses more work.
    pub max_trials: usize,
    /// Trials below this merit percentile are pruned, in `[0, 1]`.
    pub prune_percentile: f64,
    /// Do not prune until at least this many trials have completed.
    pub min_trials_to_prune: usize,
}

impl Default for StudyConfig {
    fn default() -> Self {
        Self {
            max_trials: 1_000,
            prune_percentile: 0.25,
            min_trials_to_prune: 8,
        }
    }
}

impl StudyConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_trials == 0 {
            return Err("study needs max_trials >= 1".into());
        }
        if !self.prune_percentile.is_finite() || !(0.0..=1.0).contains(&self.prune_percentile) {
            return Err("study prune_percentile must be in [0, 1]".into());
        }
        Ok(())
    }
}

/// A trial returned by [`Campaign::next_trial`].
#[derive(Clone, Debug)]
pub struct PendingTrial {
    pub id: u64,
    pub params: BTreeMap<String, f64>,
    pub source: ProposalSource,
}

/// The campaign ledger and proposal queue.
#[derive(Clone, Debug)]
pub struct Campaign {
    cfg: StudyConfig,
    next_id: u64,
    queue: VecDeque<Proposal>,
    records: Vec<TrialRecord>,
    pins: BTreeMap<String, f64>,
}

impl Campaign {
    pub fn new(cfg: StudyConfig) -> Result<Self, String> {
        cfg.validate()?;
        Ok(Self {
            cfg,
            next_id: 0,
            queue: VecDeque::new(),
            records: Vec::new(),
            pins: BTreeMap::new(),
        })
    }

    pub fn config(&self) -> &StudyConfig {
        &self.cfg
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn records(&self) -> &[TrialRecord] {
        &self.records
    }

    pub fn queued(&self) -> usize {
        self.queue.len()
    }

    /// Queue one parameter set. The returned id is stable for the campaign.
    pub fn enqueue(
        &mut self,
        params: BTreeMap<String, f64>,
        source: ProposalSource,
    ) -> Result<u64, String> {
        if params.is_empty() {
            return Err("study cannot enqueue an empty parameter set".into());
        }
        if params.values().any(|value| !value.is_finite()) {
            return Err("study parameters must be finite".into());
        }
        if self.completed_count() >= self.cfg.max_trials {
            return Err(format!(
                "study reached max_trials = {}",
                self.cfg.max_trials
            ));
        }
        let id = self.next_id;
        self.next_id += 1;
        self.queue.push_back(Proposal { id, params, source });
        Ok(id)
    }

    /// Enqueue several proposals in order.
    pub fn enqueue_many(
        &mut self,
        proposals: impl IntoIterator<Item = BTreeMap<String, f64>>,
        source: ProposalSource,
    ) -> Result<Vec<u64>, String> {
        proposals
            .into_iter()
            .map(|params| self.enqueue(params, source))
            .collect()
    }

    /// Fix a parameter for every future proposal. A pin wins over the queued
    /// value and is recorded in the returned trial.
    pub fn pin(&mut self, name: impl Into<String>, value: f64) -> Result<(), String> {
        if !value.is_finite() {
            return Err("study pin value must be finite".into());
        }
        let name = name.into();
        if name.is_empty() {
            return Err("study pin name must not be empty".into());
        }
        self.pins.insert(name, value);
        Ok(())
    }

    pub fn unpin(&mut self, name: &str) -> Option<f64> {
        self.pins.remove(name)
    }

    pub fn pins(&self) -> &BTreeMap<String, f64> {
        &self.pins
    }

    /// Return the next queued trial with pins applied, or `None` when the
    /// queue is empty.
    pub fn next_trial(&mut self) -> Option<PendingTrial> {
        let mut proposal = self.queue.pop_front()?;
        for (name, &value) in &self.pins {
            proposal.params.insert(name.clone(), value);
        }
        let record = TrialRecord {
            id: proposal.id,
            params: proposal.params.clone(),
            source: proposal.source,
            state: TrialState::Running,
            merit: None,
        };
        self.records.push(record);
        Some(PendingTrial {
            id: proposal.id,
            params: proposal.params,
            source: proposal.source,
        })
    }

    /// Record the merit of a running trial. A non-finite merit is rejected so
    /// decode/evaluate failures cannot silently become archive winners.
    pub fn record(&mut self, id: u64, merit: f64) -> Result<(), String> {
        if !merit.is_finite() {
            return Err(format!("study trial {id} has a non-finite merit"));
        }
        let record = self
            .records
            .iter_mut()
            .find(|record| record.id == id)
            .ok_or_else(|| format!("study trial {id} is not running"))?;
        if record.state != TrialState::Running {
            return Err(format!("study trial {id} is already {:?}", record.state));
        }
        record.merit = Some(merit);
        record.state = TrialState::Complete;
        Ok(())
    }

    /// Mark a running trial as failed/unusable without inventing a merit.
    pub fn abandon(&mut self, id: u64) -> Result<(), String> {
        let record = self
            .records
            .iter_mut()
            .find(|record| record.id == id)
            .ok_or_else(|| format!("study trial {id} is not running"))?;
        record.state = TrialState::Pruned;
        Ok(())
    }

    /// Enqueue a completed trial's parameters again (e.g. a migrant).
    pub fn requeue(&mut self, id: u64, source: ProposalSource) -> Result<u64, String> {
        let params = self
            .records
            .iter()
            .find(|record| record.id == id)
            .map(|record| record.params.clone())
            .ok_or_else(|| format!("study trial {id} does not exist"))?;
        self.enqueue(params, source)
    }

    /// Completed trials only, in completion order.
    pub fn completed(&self) -> impl Iterator<Item = &TrialRecord> {
        self.records
            .iter()
            .filter(|record| record.state == TrialState::Complete)
    }

    pub fn completed_count(&self) -> usize {
        self.completed().count()
    }

    /// Lowest-merit completed trial.
    pub fn best(&self) -> Option<&TrialRecord> {
        self.completed()
            .min_by(|a, b| a.merit.unwrap().total_cmp(&b.merit.unwrap()))
    }

    /// Prune completed trials below the configured percentile. At least
    /// `min_trials_to_prune` completed records are required before this can
    /// remove anything. Returns the ids that were pruned.
    pub fn prune_dead(&mut self) -> Vec<u64> {
        if self.completed_count() < self.cfg.min_trials_to_prune {
            return Vec::new();
        }
        let mut merits: Vec<f64> = self.completed().filter_map(|record| record.merit).collect();
        merits.sort_by(f64::total_cmp);
        // Keep the better `1 - prune_percentile` fraction; a minimisation
        // campaign prunes the high-merit tail, never the best designs.
        let keep = (((merits.len() as f64) * (1.0 - self.cfg.prune_percentile)).ceil() as usize)
            .clamp(1, merits.len());
        if keep == merits.len() {
            return Vec::new();
        }
        let cutoff = merits[keep];
        let mut pruned = Vec::new();
        for record in &mut self.records {
            if record.state == TrialState::Complete && record.merit.is_some_and(|m| m >= cutoff) {
                record.state = TrialState::Pruned;
                pruned.push(record.id);
            }
        }
        pruned
    }

    /// Remove pruned/failed records from the ledger. Completed data is never
    /// discarded, because the study's purpose is the ledger.
    pub fn compact(&mut self) -> usize {
        let before = self.records.len();
        self.records
            .retain(|record| record.state != TrialState::Pruned);
        before - self.records.len()
    }

    /// One-way fANOVA-style importance over completed trials: the between-bin
    /// variance of the mean merit for each parameter. This is a deterministic
    /// explanation aid, not a replacement for the external fANOVA crate; it
    /// needs only the ledger and no new solves.
    pub fn importance(&self) -> BTreeMap<String, f64> {
        let completed: Vec<&TrialRecord> = self.completed().collect();
        let mut out = BTreeMap::new();
        if completed.len() < 2 {
            return out;
        }
        let grand = completed.iter().filter_map(|r| r.merit).sum::<f64>() / completed.len() as f64;
        let mut names: Vec<&str> = completed
            .iter()
            .flat_map(|record| record.params.keys().map(String::as_str))
            .collect();
        names.sort_unstable();
        names.dedup();
        for name in names {
            let mut groups: BTreeMap<u64, (f64, usize)> = BTreeMap::new();
            for record in &completed {
                let Some(&value) = record.params.get(name) else {
                    continue;
                };
                // Quantise to a stable bin. The exact bin width is not a
                // statistical claim; it only keeps continuous parameters from
                // creating one group per trial.
                let bin = (value * 1_000.0).round() as i64 as u64;
                let entry = groups.entry(bin).or_insert((0.0, 0));
                entry.0 += record.merit.unwrap();
                entry.1 += 1;
            }
            if groups.len() < 2 {
                continue;
            }
            let between = groups
                .values()
                .map(|(sum, count)| {
                    let mean = sum / *count as f64;
                    *count as f64 * (mean - grand).powi(2)
                })
                .sum::<f64>()
                / completed.len() as f64;
            out.insert(name.to_string(), between);
        }
        out
    }

    /// Convenience: enqueue a set of parameter maps and evaluate them through
    /// one closure. This is the queue contract used by the study examples.
    pub fn run_batch<F>(
        &mut self,
        proposals: impl IntoIterator<Item = BTreeMap<String, f64>>,
        source: ProposalSource,
        mut evaluate: F,
    ) -> Result<Vec<u64>, String>
    where
        F: FnMut(&BTreeMap<String, f64>) -> Result<f64, String>,
    {
        let ids = self.enqueue_many(proposals, source)?;
        for _ in &ids {
            let Some(trial) = self.next_trial() else {
                break;
            };
            let merit = evaluate(&trial.params)?;
            self.record(trial.id, merit)?;
        }
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
        pairs
            .iter()
            .map(|(name, value)| (name.to_string(), *value))
            .collect()
    }

    #[test]
    fn pins_win_over_queued_values() {
        let mut campaign = Campaign::new(StudyConfig::default()).unwrap();
        campaign.pin("substrate", 1.52).unwrap();
        campaign
            .enqueue(
                params(&[("substrate", 1.0), ("d", 100.0)]),
                ProposalSource::Ga,
            )
            .unwrap();
        let trial = campaign.next_trial().unwrap();
        assert_eq!(trial.params["substrate"], 1.52);
        campaign.record(trial.id, 1.0).unwrap();
        assert_eq!(campaign.best().unwrap().merit, Some(1.0));
    }

    #[test]
    fn pruning_needs_the_minimum_and_keeps_the_best() {
        let mut campaign = Campaign::new(StudyConfig {
            max_trials: 100,
            prune_percentile: 0.5,
            min_trials_to_prune: 4,
        })
        .unwrap();
        for merit in [4.0, 3.0, 2.0, 1.0] {
            campaign
                .enqueue(params(&[("x", merit)]), ProposalSource::Ga)
                .unwrap();
            let trial = campaign.next_trial().unwrap();
            campaign.record(trial.id, merit).unwrap();
        }
        let pruned = campaign.prune_dead();
        assert_eq!(pruned.len(), 2);
        assert_eq!(campaign.best().unwrap().merit, Some(1.0));
        assert_eq!(campaign.completed_count(), 2);
        assert_eq!(campaign.importance().len(), 1);
    }

    #[test]
    fn requeue_and_compact_preserve_the_ledger() {
        let mut campaign = Campaign::new(StudyConfig::default()).unwrap();
        let id = campaign
            .enqueue(params(&[("x", 1.0)]), ProposalSource::Archive)
            .unwrap();
        let trial = campaign.next_trial().unwrap();
        campaign.record(trial.id, 0.5).unwrap();
        let again = campaign.requeue(id, ProposalSource::Ga).unwrap();
        assert_ne!(id, again);
        assert_eq!(campaign.queued(), 1);
        assert_eq!(campaign.completed_count(), 1);
        assert_eq!(campaign.compact(), 0);
    }

    #[test]
    fn a_non_finite_result_is_refused() {
        let mut campaign = Campaign::new(StudyConfig::default()).unwrap();
        campaign
            .enqueue(params(&[("x", 1.0)]), ProposalSource::Ga)
            .unwrap();
        let trial = campaign.next_trial().unwrap();
        assert!(campaign.record(trial.id, f64::NAN).is_err());
        assert!(campaign.record(999, 1.0).is_err());
    }
}
