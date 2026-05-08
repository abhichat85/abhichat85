//! Cognitive memory primitives.
//!
//! `MemoryObject` is Prajna's fundamental unit of storage — the equivalent of
//! a row in a relational database, but shaped for cognition rather than
//! transactions. Every fact, episode, goal, strategy, and identity model an
//! agent carries is a `MemoryObject`.
//!
//! # Memory hierarchy
//!
//! ```text
//! Tier 0  Raw events          (decay fast, high volume)
//! Tier 1  Sessions            (daily groupings)
//! Tier 2  Episodes            (goal-bounded experiences)
//! Tier 3  Patterns            (recurring behaviours)
//! Tier 4  Strategies          (high-level heuristics)
//! Tier 5  Identity models     (persistent self-representation)
//! Tier 6  World models        (long-horizon environmental beliefs)
//! ```
//!
//! Compression promotes memories up the hierarchy. Higher tiers decay more
//! slowly and are weighted more heavily during context compilation.

pub mod salience;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── MemoryKind ───────────────────────────────────────────────────────────────

/// The semantic type of a memory object.
///
/// The kind informs compression, retrieval weighting, and context placement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    /// A bounded experience: what the agent did, what happened, what it learned.
    Episode,
    /// A declarative fact or piece of structured world knowledge.
    Fact,
    /// An objective the agent is pursuing or has pursued.
    Goal,
    /// A low-level execution trace (tool calls, sub-steps, intermediate outputs).
    Trace,
    /// A recurring behavioural pattern extracted across episodes.
    Pattern,
    /// A generalised heuristic for achieving a class of goals.
    Strategy,
    /// A compressed high-order belief distilled from lower-tier memories.
    Abstraction,
    /// A persistent self-representation (preferences, tendencies, identity).
    IdentityKernel,
    /// A high-level belief about how the world or an environment works.
    WorldModel,
}

// ── MemoryObject ─────────────────────────────────────────────────────────────

/// A single cognitive memory — Prajna's primary storage unit.
///
/// A `MemoryObject` is richer than a document chunk and richer than a row.
/// It carries:
///
/// - **Semantic embedding**: for similarity-based retrieval.
/// - **Temporal metadata**: creation, access, and reinforcement timestamps.
/// - **Salience signals**: access count, reinforcement count, base salience.
/// - **Epistemic metadata**: confidence, provenance, contradiction links.
/// - **Graph edges**: causal parents, compressed-from lineage.
/// - **Goal associations**: which agent goals this memory is relevant to.
///
/// All fields are `serde`-compatible for persistence via the storage engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryObject {
    /// Globally unique identifier for this memory.
    pub id: Uuid,
    /// The agent that owns this memory.
    pub agent_id: Uuid,
    /// Semantic kind of this memory.
    pub kind: MemoryKind,

    // ── Content ──────────────────────────────────────────────────────────────

    /// Human-readable or structured content. For large blobs this is a
    /// summary; the full payload is stored in an overflow chain.
    pub content: String,
    /// Semantic embedding vector. Empty until the embedding model is called.
    /// Dimensionality is model-dependent (768, 1536, 3072, …).
    pub embedding: Vec<f32>,

    // ── Memory hierarchy ─────────────────────────────────────────────────────

    /// Compression tier (0 = raw event, 6 = world model).
    pub compression_tier: u8,

    // ── Temporal ─────────────────────────────────────────────────────────────

    /// Unix timestamp (seconds) when this memory was first created.
    pub created_at: i64,
    /// Unix timestamp of the most recent read or retrieval.
    pub last_accessed_at: i64,
    /// Unix timestamp of the most recent reinforcement event.
    pub last_reinforced_at: i64,

    // ── Salience signals ─────────────────────────────────────────────────────

    /// Total number of times this memory has been retrieved.
    pub access_count: u32,
    /// Total number of reinforcement events (agent acted on this memory and
    /// the outcome was positive or notable).
    pub reinforcement_count: u32,
    /// Base salience score before decay and reinforcement boost are applied.
    /// Modified by `reinforce()` and explicit override.
    pub base_salience: f32,

    // ── Epistemic ────────────────────────────────────────────────────────────

    /// Confidence that this memory is accurate (0.0–1.0).
    /// Propagates to dependent abstractions when updated.
    pub confidence: f32,

    // ── Relationships ────────────────────────────────────────────────────────

    /// Goal tags: short string identifiers of goals this memory is relevant to.
    /// Used by goal-conditioned retrieval to pre-filter candidates.
    pub goal_tags: Vec<String>,
    /// IDs of memories that causally preceded this one.
    pub causal_parent_ids: Vec<Uuid>,
    /// IDs of lower-tier memories this was compressed from.
    pub compressed_from_ids: Vec<Uuid>,
    /// IDs of memories this one contradicts.
    pub contradicts_ids: Vec<Uuid>,
    /// Named entity references (people, systems, concepts) mentioned.
    pub entity_refs: Vec<String>,
}

impl MemoryObject {
    /// Create a new memory with sensible defaults.
    ///
    /// The embedding is left empty; call your embedding model and populate
    /// `memory.embedding` before storing.
    pub fn new(agent_id: Uuid, kind: MemoryKind, content: impl Into<String>) -> Self {
        let now = now_unix();
        Self {
            id:                    Uuid::new_v4(),
            agent_id,
            kind,
            content:               content.into(),
            embedding:             Vec::new(),
            compression_tier:      0,
            created_at:            now,
            last_accessed_at:      now,
            last_reinforced_at:    now,
            access_count:          0,
            reinforcement_count:   0,
            base_salience:         0.5,
            confidence:            1.0,
            goal_tags:             Vec::new(),
            causal_parent_ids:     Vec::new(),
            compressed_from_ids:   Vec::new(),
            contradicts_ids:       Vec::new(),
            entity_refs:           Vec::new(),
        }
    }

    /// Record a retrieval access.
    pub fn touch(&mut self) {
        self.last_accessed_at = now_unix();
        self.access_count     = self.access_count.saturating_add(1);
    }

    /// Record a positive reinforcement event.
    ///
    /// Raises `base_salience` by 10%, capped at 1.0. Repeatedly reinforced
    /// memories asymptotically approach maximum salience.
    pub fn reinforce(&mut self) {
        self.last_reinforced_at    = now_unix();
        self.reinforcement_count   = self.reinforcement_count.saturating_add(1);
        self.base_salience         = (self.base_salience * 1.1).min(1.0);
    }

    /// Promote this memory to `new_tier` (compression step).
    ///
    /// Higher tiers decay more slowly and survive context compression longer.
    /// The `source_ids` are the memories that were compressed into this one.
    pub fn promote(&mut self, new_tier: u8, source_ids: Vec<Uuid>) {
        self.compression_tier      = new_tier;
        self.compressed_from_ids   = source_ids;
        // Bump base salience on promotion — compressed knowledge is assumed
        // to be higher-value than its raw constituents.
        self.base_salience         = (self.base_salience * 1.2).min(1.0);
    }

    /// Declare that this memory contradicts `other_id`.
    pub fn add_contradiction(&mut self, other_id: Uuid) {
        if !self.contradicts_ids.contains(&other_id) {
            self.contradicts_ids.push(other_id);
        }
    }

    /// Return the current salience of this memory at `now` (unix seconds).
    ///
    /// This is a convenience wrapper around [`salience::current`].
    pub fn current_salience(&self, now: i64) -> f32 {
        salience::current(self, now)
    }

    /// Return `true` if this memory has an embedding vector.
    pub fn is_embedded(&self) -> bool {
        !self.embedding.is_empty()
    }
}

// ── Utilities ────────────────────────────────────────────────────────────────

pub(crate) fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn agent() -> Uuid {
        Uuid::new_v4()
    }

    #[test]
    fn new_memory_has_defaults() {
        let m = MemoryObject::new(agent(), MemoryKind::Episode, "agent launched a search");
        assert_eq!(m.compression_tier, 0);
        assert_eq!(m.access_count, 0);
        assert!(!m.is_embedded());
        assert!((m.base_salience - 0.5).abs() < 1e-6);
        assert!((m.confidence - 1.0).abs() < 1e-6);
    }

    #[test]
    fn reinforce_raises_salience() {
        let mut m = MemoryObject::new(agent(), MemoryKind::Fact, "reinforcement raises salience");
        let s0 = m.base_salience;
        m.reinforce();
        assert!(m.base_salience > s0);
        assert_eq!(m.reinforcement_count, 1);
    }

    #[test]
    fn reinforce_caps_at_one() {
        let mut m = MemoryObject::new(agent(), MemoryKind::Strategy, "cap test");
        for _ in 0..100 {
            m.reinforce();
        }
        assert!(m.base_salience <= 1.0);
    }

    #[test]
    fn promote_sets_tier_and_lineage() {
        let sources = vec![Uuid::new_v4(), Uuid::new_v4()];
        let mut m = MemoryObject::new(agent(), MemoryKind::Episode, "raw event");
        m.promote(2, sources.clone());
        assert_eq!(m.compression_tier, 2);
        assert_eq!(m.compressed_from_ids, sources);
    }

    #[test]
    fn touch_increments_access_count() {
        let mut m = MemoryObject::new(agent(), MemoryKind::Fact, "touched");
        m.touch();
        m.touch();
        assert_eq!(m.access_count, 2);
    }

    #[test]
    fn contradiction_deduplicated() {
        let other = Uuid::new_v4();
        let mut m = MemoryObject::new(agent(), MemoryKind::Fact, "contradictory belief");
        m.add_contradiction(other);
        m.add_contradiction(other);
        assert_eq!(m.contradicts_ids.len(), 1);
    }
}
