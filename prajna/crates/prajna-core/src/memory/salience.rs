//! Salience scoring for cognitive memories.
//!
//! # The Salience Model
//!
//! Every `MemoryObject` has a *current salience* — a scalar in `[0, 1]` that
//! represents how cognitively important this memory is right now. Salience
//! drives three critical decisions:
//!
//! 1. **Buffer pool eviction** — high-salience pages stay in RAM.
//! 2. **Context compilation** — high-salience memories get more of the token
//!    budget when the Context Compiler assembles a cognitive state packet.
//! 3. **Compression scheduling** — low-salience memories are compressed first.
//!
//! # Formula
//!
//! ```text
//! salience(m, t) = base_salience(m)
//!                 × decay_rate(tier)^age_days(m, t)
//!                 × reinforcement_boost(m)
//! ```
//!
//! Where:
//! - `base_salience` starts at 0.5 and is raised by each `reinforce()` call.
//! - `decay_rate` depends on the compression tier (tier 0 decays fast,
//!   tier 6 is nearly permanent).
//! - `reinforcement_boost = 1 + ln(reinforcement_count + 1)` — log-scaled
//!   so that a small number of reinforcements gives a significant lift, but
//!   the boost saturates gracefully.
//!
//! # Goal-Conditioned Ranking
//!
//! `rank_by_goal` combines salience with semantic relevance to a goal
//! embedding. The blend weight (0.4 salience / 0.6 relevance) is configurable
//! and will become a learned parameter in a later phase.

use super::MemoryObject;

// ── Decay rates per tier ─────────────────────────────────────────────────────

/// Per-tier daily decay rates used in `salience = base × rate^age_days`.
///
/// A rate closer to 1.0 means slower decay (longer half-life).
/// Higher compression tiers represent more distilled, durable knowledge
/// and therefore decay more slowly.
///
/// Half-lives (days until salience halves from its base value):
/// tier 0: ~14 d | tier 1: ~23 d | tier 2: ~35 d | tier 3: ~69 d
/// tier 4: ~138 d (~4.6 mo) | tier 5: ~693 d (~1.9 yr) | tier 6: ~1386 d (~3.8 yr)
const DECAY_RATES: [f32; 7] = [
    0.9500, // tier 0 — raw events:      half-life ≈ 14 days
    0.9700, // tier 1 — sessions:        half-life ≈ 23 days
    0.9800, // tier 2 — episodes:        half-life ≈ 35 days
    0.9900, // tier 3 — patterns:        half-life ≈ 69 days
    0.9950, // tier 4 — strategies:      half-life ≈ 138 days
    0.9990, // tier 5 — identity models: half-life ≈ 693 days
    0.9995, // tier 6 — world models:    half-life ≈ 1386 days
];

// ── Core salience function ───────────────────────────────────────────────────

/// Compute the current salience of `memory` at `now_unix` (seconds).
///
/// The result is *not* clamped to `[0, 1]` — `reinforcement_boost` can push
/// it above 1.0 for heavily reinforced memories. Callers that need a bounded
/// value should call `.min(1.0)` themselves.
pub fn current(memory: &MemoryObject, now_unix: i64) -> f32 {
    let tier       = memory.compression_tier.min(6) as usize;
    let decay_rate = DECAY_RATES[tier];
    let age_days   = age_days(memory.created_at, now_unix);
    let boost      = reinforcement_boost(memory.reinforcement_count);

    memory.base_salience * decay_rate.powf(age_days) * boost
}

/// Age of a memory in fractional days.
fn age_days(created_at: i64, now: i64) -> f32 {
    (now - created_at).max(0) as f32 / 86_400.0
}

/// Log-scaled reinforcement boost.
///
/// `boost(0)  = 1.0`  (no reinforcement → no boost)
/// `boost(1)  ≈ 1.69`
/// `boost(9)  ≈ 3.30`
/// `boost(99) ≈ 5.61`
fn reinforcement_boost(count: u32) -> f32 {
    1.0 + (count as f32 + 1.0).ln()
}

// ── Goal-conditioned ranking ─────────────────────────────────────────────────

/// Blend weight for salience vs. goal relevance in `rank_by_goal`.
const SALIENCE_WEIGHT:   f32 = 0.4;
const RELEVANCE_WEIGHT:  f32 = 0.6;

/// Score each memory by a linear blend of its current salience and its
/// cosine similarity to `goal_embedding`, then return indices sorted
/// descending (highest-scoring first).
///
/// Memories without an embedding receive a relevance score of 0.0 and are
/// ranked by salience alone.
///
/// This is the core ranking function for the Context Compiler (v1, greedy).
pub fn rank_by_goal(
    memories:       &[MemoryObject],
    goal_embedding: &[f32],
    now_unix:       i64,
) -> Vec<usize> {
    let mut scored: Vec<(usize, f32)> = memories
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let s = current(m, now_unix);
            let r = if goal_embedding.is_empty() || m.embedding.is_empty() {
                0.0
            } else {
                cosine_similarity(&m.embedding, goal_embedding).max(0.0)
            };
            let score = SALIENCE_WEIGHT * s + RELEVANCE_WEIGHT * r;
            (i, score)
        })
        .collect();

    scored.sort_unstable_by(|(_, a), (_, b)| {
        b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)
    });

    scored.into_iter().map(|(i, _)| i).collect()
}

// ── Maximal Marginal Relevance ───────────────────────────────────────────────

/// Select up to `k` diverse, high-scoring memories from `candidates` using
/// Maximal Marginal Relevance (MMR).
///
/// MMR balances relevance to `goal_embedding` against redundancy with
/// already-selected memories, avoiding the failure mode of selecting many
/// nearly-identical episodes.
///
/// `lambda`: trade-off weight (1.0 = pure relevance, 0.0 = pure diversity).
///
/// Returns a `Vec` of indices into `candidates`, up to length `k`.
pub fn mmr_select(
    candidates:     &[&MemoryObject],
    goal_embedding: &[f32],
    now_unix:       i64,
    k:              usize,
    lambda:         f32,
) -> Vec<usize> {
    if candidates.is_empty() || k == 0 {
        return Vec::new();
    }

    let mut selected: Vec<usize>  = Vec::with_capacity(k);
    let mut remaining: Vec<usize> = (0..candidates.len()).collect();

    while selected.len() < k && !remaining.is_empty() {
        let best_idx = remaining
            .iter()
            .enumerate()
            .map(|(ri, &ci)| {
                let m         = candidates[ci];
                let salience  = current(m, now_unix);
                let relevance = if goal_embedding.is_empty() || m.embedding.is_empty() {
                    0.0_f32
                } else {
                    cosine_similarity(&m.embedding, goal_embedding).max(0.0)
                };
                let relevance_score = SALIENCE_WEIGHT * salience + RELEVANCE_WEIGHT * relevance;

                let max_redundancy = selected
                    .iter()
                    .map(|&si| {
                        let sm = candidates[si];
                        if m.embedding.is_empty() || sm.embedding.is_empty() {
                            0.0_f32
                        } else {
                            cosine_similarity(&m.embedding, &sm.embedding).max(0.0)
                        }
                    })
                    .fold(0.0_f32, f32::max);

                let mmr = lambda * relevance_score - (1.0 - lambda) * max_redundancy;
                (ri, mmr)
            })
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(ri, _)| ri);

        match best_idx {
            Some(ri) => {
                let ci = remaining.remove(ri);
                selected.push(ci);
            }
            None => break,
        }
    }

    selected
}

// ── Vector math ──────────────────────────────────────────────────────────────

/// Cosine similarity between two vectors of equal length.
/// Returns 0.0 if either vector is the zero vector.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "embedding dimension mismatch");

    let mut dot    = 0.0_f32;
    let mut norm_a = 0.0_f32;
    let mut norm_b = 0.0_f32;

    for (x, y) in a.iter().zip(b.iter()) {
        dot    += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < f32::EPSILON {
        0.0
    } else {
        (dot / denom).clamp(-1.0, 1.0)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryKind, MemoryObject};
    use uuid::Uuid;

    fn make_memory(kind: MemoryKind, tier: u8, base_salience: f32, age_days: f32) -> MemoryObject {
        let now = 1_700_000_000_i64;
        let created_at = now - (age_days * 86_400.0) as i64;
        let mut m = MemoryObject::new(Uuid::new_v4(), kind, "test");
        m.compression_tier = tier;
        m.base_salience    = base_salience;
        m.created_at       = created_at;
        m
    }

    #[test]
    fn higher_tier_decays_slower() {
        let now      = 1_700_000_000_i64;
        let tier0    = make_memory(MemoryKind::Episode,  0, 1.0, 7.0);
        let tier4    = make_memory(MemoryKind::Strategy, 4, 1.0, 7.0);

        let s0 = current(&tier0, now);
        let s4 = current(&tier4, now);
        assert!(s4 > s0, "tier-4 strategy should outlast tier-0 episode after 7 days");
    }

    #[test]
    fn fresh_memory_near_base_salience() {
        let now = 1_700_000_000_i64;
        let m   = make_memory(MemoryKind::Fact, 0, 0.5, 0.0);
        let s   = current(&m, now);
        // No decay (age=0), reinforcement_boost(0) = ln(1) + 1 = 1.0
        assert!((s - 0.5).abs() < 1e-4);
    }

    #[test]
    fn reinforcement_raises_effective_salience() {
        let now = 1_700_000_000_i64;
        let mut m = make_memory(MemoryKind::Episode, 0, 0.5, 0.0);

        let before = current(&m, now);
        for _ in 0..5 { m.reinforce(); }
        let after = current(&m, now);

        assert!(after > before);
    }

    #[test]
    fn cosine_identical_vectors() {
        let v = vec![1.0_f32, 2.0, 3.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = vec![1.0_f32, 0.0, 0.0];
        let b = vec![0.0_f32, 1.0, 0.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn rank_by_goal_orders_by_combined_score() {
        let now  = 1_700_000_000_i64;
        let goal = vec![1.0_f32, 0.0];

        let mut m_relevant = make_memory(MemoryKind::Fact, 0, 0.5, 0.0);
        m_relevant.embedding = vec![1.0, 0.0]; // perfectly aligned with goal

        let mut m_irrelevant = make_memory(MemoryKind::Fact, 0, 0.5, 0.0);
        m_irrelevant.embedding = vec![0.0, 1.0]; // orthogonal to goal

        let memories = vec![m_irrelevant, m_relevant]; // irrelevant is index 0
        let ranked   = rank_by_goal(&memories, &goal, now);

        assert_eq!(ranked[0], 1, "relevant memory should rank first");
    }

    #[test]
    fn mmr_selects_diverse_memories() {
        let now  = 1_700_000_000_i64;
        let goal = vec![1.0_f32, 0.0, 0.0];

        let mut m0 = make_memory(MemoryKind::Episode, 0, 0.8, 0.0);
        m0.embedding = vec![1.0, 0.0, 0.0];

        let mut m1 = make_memory(MemoryKind::Episode, 0, 0.8, 0.0);
        m1.embedding = vec![1.0, 0.01, 0.0]; // almost identical to m0

        let mut m2 = make_memory(MemoryKind::Episode, 0, 0.7, 0.0);
        m2.embedding = vec![0.0, 1.0, 0.0]; // orthogonal — diverse

        let candidates: Vec<&MemoryObject> = vec![&m0, &m1, &m2];
        let selected = mmr_select(&candidates, &goal, now, 2, 0.5);

        assert_eq!(selected.len(), 2);
        // m2 (diverse) should be preferred over m1 (near-duplicate of m0)
        assert!(selected.contains(&0), "most relevant should always be selected first");
        assert!(
            selected.contains(&2),
            "diverse candidate should beat near-duplicate"
        );
    }
}
