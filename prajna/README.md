<div align="center">

# Prajna

**A cognitive database engine, built from scratch, for AI agents.**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](./LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-dea584.svg)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/status-early%20development-orange.svg)]()

</div>

---

Prajna is not a vector database.
It is not a wrapper around an existing storage engine.
It is not a RAG framework or a memory middleware.

It is a **native cognitive storage engine** — built from the page layer up —
for the workloads that AI agents actually have: temporal memory, goal-conditioned
retrieval, identity continuity, and long-horizon reasoning across time.

## The Problem

Vector databases answer: *"What text is most similar to this text?"*

That is not the question agents need answered.

Agents need: *"Given what I am trying to accomplish right now, what is the optimal
cognitive state to reason from?"*

These are fundamentally different problems. Retrieval is not cognition. Prajna is
built to solve the second problem, from the storage primitives up — page layout,
buffer pool, indexes, query planner, and cognitive compiler — all designed
natively for cognitive workloads.

---

## What Makes Prajna Different

### 1. Embedding-Aware Page Layout
Every 16 KB page separates embedding vectors into a contiguous, 32-byte-aligned
region. A SIMD similarity scan reads only this region — 4–8× less I/O than
reading full records.

### 2. Salience-Weighted Buffer Pool
Instead of LRU, the buffer pool evicts based on cognitive salience. Identity
kernels and strategic memories stay in RAM. Old raw event logs fall cold first.

### 3. Cognitive MVCC
Agents can fork their memory state for speculative reasoning, then merge
trajectories back. The version control model is a DAG, not a linear chain —
closer to Git's object model than PostgreSQL's MVCC.

### 4. The Unified CogTree Index *(planned)*
One index structure supporting key lookup, vector similarity, temporal range,
and graph traversal simultaneously. No join between indexes. No cross-index
coordination at query time.

### 5. Goal-Conditioned Context Compilation *(planned)*
The Context Compiler assembles a structured cognitive state packet given a goal
and a token budget. Not a list of similar chunks — a curated working memory,
strategic context, open loops, identity priors, and contradiction alerts.

---

## Architecture

```
Applications / Agent Runtimes
           │
           ▼
  ┌─────────────────────────┐
  │   Python / TS / Go SDK   │
  └───────────┬─────────────┘
              │
  ┌───────────▼─────────────────────────────┐
  │   Context Compiler  (Prajna QL)          │  goal-conditioned state reconstruction
  ├─────────────────────────────────────────┤
  │   Salience Engine                        │  dynamic cognitive prioritisation
  ├─────────────────────────────────────────┤
  │   Cognitive Query Executor               │
  ├─────────────────────────────────────────┤
  │   CogTree  (Unified Index)               │  key · vector · temporal · graph
  ├─────────────────────────────────────────┤
  │   Transaction Manager  (Cognitive MVCC)  │  branching · merging · versioning
  ├─────────────────────────────────────────┤
  │   Write-Ahead Log                        │  durability · crash recovery
  ├─────────────────────────────────────────┤
  │   Buffer Pool                            │  salience-weighted page cache      ◄─ L2
  ├─────────────────────────────────────────┤
  │   Page Manager                           │  embedding-aware 16 KB pages       ◄─ L1
  ├─────────────────────────────────────────┤
  │   Block I/O                              │  tiered file I/O · CRC32c checks   ◄─ L0
  └─────────────────────────────────────────┘
              │
  ┌───────────▼─────────────────────────────┐
  │   Storage  (NVMe → SSD → HDD → cold)    │
  └─────────────────────────────────────────┘
```

---

## Current Status

We are building from the ground up. The cognitive layer is fully designed;
it will be built on a foundation that is correct.

| Phase | Layer | Status |
|-------|-------|--------|
| 1 | Block I/O (L0) | **done** |
| 1 | Embedding-aware page format (L1) | **done** |
| 1 | Salience-weighted buffer pool (L2) | **done (skeleton)** |
| 1 | MemoryObject type system | **done** |
| 1 | Salience model + MMR ranking | **done** |
| 1 | Write-ahead log (L3) | planned |
| 1 | Free space management (L4) | planned |
| 2 | B-tree index (L5) | planned |
| 2 | Unified CogTree index | planned |
| 2 | Cognitive MVCC | planned |
| 3 | Context Compiler | planned |
| 3 | Identity Kernel | planned |
| 3 | Memory compression pipeline | planned |
| 4 | Python / TypeScript SDKs | planned |
| 4 | Multi-agent shared memory | planned |

---

## Getting Started

Prajna is in early development. The storage engine compiles and tests pass;
higher-level APIs are not yet ready for application use.

```bash
git clone https://github.com/abhichat85/prajna
cd prajna
cargo build
cargo test
```

Requires Rust 1.75 or later.

---

## Repository Layout

```
prajna/
├── crates/
│   ├── prajna-storage/    # L0–L2: block I/O, page format, buffer pool
│   └── prajna-core/       # cognitive types: MemoryObject, salience, context
├── docs/
│   ├── PHILOSOPHY.md      # the theoretical and philosophical foundations
│   └── ROADMAP.md         # the multi-year build plan
└── bench/
    └── pcb/               # Prajna Cognitive Benchmark suite
```

---

## Design Documents

- [ARCHITECTURE.md](./ARCHITECTURE.md) — full architectural specification
- [docs/PHILOSOPHY.md](./docs/PHILOSOPHY.md) — theoretical foundations
- [docs/ROADMAP.md](./docs/ROADMAP.md) — multi-year build plan
- [bench/pcb/README.md](./bench/pcb/README.md) — cognitive benchmark suite

---

## Contributing

Prajna is a long-horizon open source infrastructure project. Contributions are
welcome at every layer.

- Issues tagged **`good-first-issue`** are suitable for contributors new to
  the codebase (SDK bindings, test coverage, documentation).
- Issues tagged **`research-needed`** involve open algorithmic questions where
  a literature review and design proposal is the contribution.
- Issues tagged **`storage-engine`** involve the core Rust systems work.

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

---

## License

Apache 2.0. See [LICENSE](./LICENSE).

---

*"Retrieval is not cognition. Prajna reconstructs cognitive state."*
