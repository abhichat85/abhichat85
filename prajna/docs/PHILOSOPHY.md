# The Philosophy of Prajna

This document explains why Prajna exists at a deeper level than engineering
requirements. It is not a technical specification; it is a statement of intent.

---

## The Name

*Prajna* (प्रज्ञा) is a Sanskrit term denoting wisdom, insight, and the
capacity to perceive the nature of things clearly. In Buddhist philosophy it
is the third component of the Noble Eightfold Path. In Vedic epistemology it
represents the highest form of knowing — not accumulated information, but
direct apprehension of reality.

The name was chosen deliberately. We are not building a better information
retrieval system. We are building the substrate through which agents come to
*understand* their situation — persistently, coherently, over time.

---

## The Vedic Architecture Mapping

Ancient Indian cognitive frameworks described the mind as layered and dynamic.
The Antahkarana (inner instrument) comprised four functions:

| Sanskrit | English approximation | Prajna analog |
|----------|-----------------------|---------------|
| Manas    | Attending mind, perception | Attention Router, context scheduler |
| Buddhi   | Discriminating intelligence | Reasoning layer, contradiction engine |
| Chitta   | Latent mind, deep memory | Persistent memory substrate |
| Ahamkara | I-maker, identity sense | Identity Kernel |
| Samskara | Impressions, conditioning | Reinforcement memory traces |
| Smriti   | Deliberate recall | Explicit retrieval layer |
| Viveka   | Discrimination, discernment | Contradiction detection |

This is not mysticism. It is pattern-matching across millennia of careful
observation of how minds work — observation that predates modern cognitive
science by two thousand years and in some respects anticipates it.

The mapping is architectural inspiration, not implementation constraint.
When we design the Identity Kernel, the Ahamkara framing is a reminder that
identity must be a first-class object with its own storage, update semantics,
and retrieval conditioning — not an afterthought bolted onto memory retrieval.

---

## Why From Scratch

Every existing database was built for a workload that is not this workload.

Relational databases were built for transactional consistency over tabular data.
Their page formats, buffer pools, query planners, and concurrency models are all
tuned for that workload. They are extraordinary at it.

Vector databases were built for semantic similarity retrieval. Their architecture
is roughly: embed documents, build an ANN index, respond to nearest-neighbour
queries. They are good at that.

Neither architecture is correct for cognition.

Cognition is:
- **Temporal** — what you know changes depending on *when* you knew it
- **Salience-driven** — not all knowledge is equally important to hold in mind
- **Identity-conditioned** — what you retrieve depends on who you are and what you want
- **Compressive** — minds summarise; they do not preserve raw experience indefinitely
- **Goal-directed** — retrieval is not similarity search; it is preparation for action

An engine that is correct for this workload cannot be retrofitted from an engine
correct for a different workload. The storage format, the buffer pool policy,
the index structure, the query language — all of them would need to be different.

So we build from scratch.

---

## The Ambition

Relational databases organised information.
Search engines organised the web.
Vector databases organised semantic similarity.

Prajna aims to organise *cognition itself* — the persistent, evolving, goal-
directed mental state of autonomous agents operating across time.

If that ambition is achieved, it will matter more than any of the above,
because autonomous agents will become the primary actors in the digital economy,
and the substrate on which they think will be the most important infrastructure
layer of the era.

This is why the project is built from the block layer up, with the same care
and rigour that went into PostgreSQL. The ambition demands no less.

---

## The Relationship Between Philosophy and Engineering

The philosophy is not a substitute for rigorous engineering. It is a compass.

When there are two valid engineering choices, the philosophy resolves the tie.
When there is a temptation to take a shortcut that would compromise correctness,
the philosophy provides the will to resist.

Every architectural decision in Prajna should be traceable to a concrete
cognitive requirement. If it isn't, it is probably unnecessary.

---

*"The goal is not better retrieval. The goal is persistent machine cognition."*
