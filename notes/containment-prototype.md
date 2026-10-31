# Containment prototype

`sundgren contain A B --base REV` measures how much of the work branch A added is found in branch
B's work, and the other way round.

```console
$ sundgren contain feature dev --base main --format text
```

## What it does

- **Work** of a branch: the lines it added since it left the base, from `git diff merge-base(base,
  branch) branch`. Each side is measured from its own fork point.
- **Normalization**: whitespace collapsed; lines without a letter or digit (blank lines, lone
  braces) dropped. Nothing language-specific: comments and license headers count.
- **Exact**: the same normalized line in B's work. Each line of B matches at most one line of A.
- **Fuzzy** (`--fuzzy T`, default 0.9): for lines with no exact match, the most similar line of
  B's work if its normalized Levenshtein similarity (rapidfuzz) is at least T. Lines of B can be
  reused. Lines whose lengths alone rule out T are skipped.
- **In order**: the longest common subsequence of the two line-hash sequences (rapidfuzz LCSseq).
  Reported, not used for the relation.
- **Containment** A in B = (exact + fuzzy) / A's lines. Both directions, overall and per file.
- **Relation** (tentative): `same` if both directions ≥ 0.8; `a-in-b`/`b-in-a` if one is; `overlap`
  if either is ≥ 0.2; else `unrelated`. `empty` if a side added nothing.

## First run on a real repository

A dev branch with about 7.5k added lines, compared with each of 49 other branches, with the main
branch as base:

- Four branches came out `a-in-b` at 82–100 %, all small: a 91-line fix at 100 %, a 546-line
  feature at 95 %, and two at 86 % and 82 %. One more scored 80 %, just under the threshold.
  These look like extractions or work shared with the dev branch.
- Eight branches had nothing ahead of their fork point (already merged): `empty`.
- **Noise floor**: most unrelated branches still scored 10–20 % in the dev branch. The matches are
  boilerplate: license header lines in every new file (one 9-line `__init__.py` matched 9/9),
  common imports and idioms, and fuzzy hits between short similar lines.
- **Speed**: exact matching took 0.05–0.1 s per pair, and 1.8 s for a branch with 469k added
  lines. Fuzzy matching is brute force over the lines without an exact match: 1–3 s for
  5–16k-line branches, and 66 s for the 469k-line one. The whole run took 84 s.

## Design questions

1. **A branch's work.** Lines added since its own fork point (now)? Or against the base's current
   tip, which would show what is *left* of a dev branch after its extractions landed? Do removed
   lines count as work?
2. **Boilerplate.** How to remove the 10–20 % floor: drop lines that already exist in the base
   tree (license headers, common imports), weight lines by how rare they are across branches,
   strip comments and headers per language as the bragi tool did, or a minimum line length?
3. **Fuzzy on short lines.** Short lines differ by a character or two and still score high.
   A minimum length for fuzzy matching, a threshold that depends on length, or comparing tokens
   instead of characters?
4. **Matching rules.** Exact matches use each line of B once, fuzzy ones can reuse a line. Should
   both use each line once?
5. **Order.** Unordered containment drives the relation; in-order is only reported. Is reordered
   code the same work?
6. **Relation thresholds.** 0.8 and 0.2 are guesses. Pick them from data, or report numbers only?
7. **Unit.** Lines only. Hunks or symbols (functions, classes) later?
8. **Scale.** Fuzzy matching needs a prefilter (an n-gram index, or comparing only within the same
   or a similarly named file) before it can handle big branches or many pairs.
9. **Pairs or all branches.** One pair per run now. A matrix over all branches against one
   base is the natural next shape.
10. **Inputs.** Branches only. Uncommitted worktrees too?
