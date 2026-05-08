# PCB — Prajna Cognitive Benchmark

The **Prajna Cognitive Benchmark** is a suite of evaluation tasks for agent
memory systems. It is designed to measure what vector databases and RAG
pipelines systematically fail at.

PCB is intentionally **system-agnostic** — it can be run against any agent
memory backend (Prajna, Letta, Mem0, Zep, plain vector DB + RAG, or no memory
at all). This makes it useful as an independent research contribution.

---

## Why Existing Benchmarks Are Insufficient

Standard NLP and retrieval benchmarks (MTEB, BEIR, MS-MARCO) measure document
similarity and passage retrieval. They are not designed to evaluate:

- **Long-horizon recall** — can the agent remember something from 50 sessions ago?
- **Goal continuity** — does the agent still pursue an unresolved objective?
- **Identity stability** — does the agent's reasoning style remain coherent over time?
- **Contradiction detection** — does the system surface conflicting beliefs?
- **Compression quality** — does summarisation preserve decision-relevant information?
- **Token efficiency** — how much task value does the agent get per context token?

PCB measures all of these.

---

## Benchmark Tasks

### PCB-1: Long-Horizon Recall
**What it tests:** Can the system retrieve a specific fact introduced N sessions ago?

Setup: inject a piece of information (a name, a decision, a constraint) at
session 0. Ask for it at session N (N = 5, 10, 25, 50, 100).

Metric: recall rate by N.

Baseline: flat RAG collapses past N≈10 due to context noise.

---

### PCB-2: Open Loop Persistence
**What it tests:** Does the system remember unresolved objectives?

Setup: agent is given a task it cannot complete in one session. Run 10
subsequent sessions on unrelated tasks. Ask the agent what it was originally
trying to do.

Metric: fraction of sessions where the open task is surfaced in context.

---

### PCB-3: Identity Stability
**What it tests:** Does the agent's decision-making remain consistent over time?

Setup: present the same decision scenario at sessions 1, 10, 25, and 50.
Evaluate whether the agent's reasoning is consistent with its prior choices
and stated preferences.

Metric: decision consistency score (human or LLM-judged).

---

### PCB-4: Contradiction Detection
**What it tests:** Does the system surface a belief that now contradicts a prior belief?

Setup: establish belief A at session 0. At session N, introduce belief ¬A
(directly or via new evidence). Evaluate whether the contradiction appears
in the context compilation.

Metric: detection rate by N and belief separation.

---

### PCB-5: Compression Quality
**What it tests:** Does memory compression preserve task-relevant information?

Setup: run 20 sessions, then compress to tier 2. Evaluate task performance
using compressed vs. raw memories. Measure information loss.

Metric: task completion rate degradation (target: < 10% at > 50% token reduction).

---

### PCB-6: Token Efficiency
**What it tests:** How much task value per context token?

Setup: identical task, varying token budgets (2k, 4k, 8k, 16k, 32k).
Measure task quality at each budget level.

Metric: task quality / token count (reasoning utility per token).

---

### PCB-7: Multi-Agent Coordination
**What it tests:** Can two agents share evolving beliefs correctly?

Setup: agent A and agent B work on different subsets of a task. At checkpoint N,
agent B must answer a question that requires agent A's findings.

Metric: cross-agent recall rate.

---

## Baselines

Each PCB task is run against:

| System | Type |
|--------|------|
| No memory | Baseline: stateless agent |
| Plain RAG | Vector DB + top-k retrieval |
| Letta (MemGPT) | Hierarchical memory framework |
| Mem0 | Memory middleware |
| **Prajna** | Cognitive database (target) |

---

## Status

PCB is under active design. Implementations will be added as Prajna reaches
the context compilation phase (Phase 3).

Contributions welcome:
- New benchmark task designs
- Implementations against existing systems
- Evaluation methodology improvements

Open an issue tagged `benchmark` to propose a new task.

---

## Citation

If you use PCB in research, please cite:

```
@misc{prajna_pcb_2026,
  title  = {PCB: Prajna Cognitive Benchmark for Agent Memory Systems},
  author = {Prajna Contributors},
  year   = {2026},
  url    = {https://github.com/abhichat85/prajna}
}
```
