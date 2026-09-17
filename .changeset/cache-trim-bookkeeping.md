---
reveal: patch
---

Trim cache bookkeeping overhead: yield prefetch offsets from an iterator instead of a fresh `Vec`, drop the unbounded `ImageCache::sizes` map in favour of the directory's bounded size hints, filter cancel sets with hash sets, and evict the cheapest entry without cloning every key and sorting.
