---
orphan: true
---

# Origin

> [!NOTE]
> A prototype.

When files were copied from another repository, and the two have moved on since, git can't say
where a file came from. `sundgren origin` compares by content: for each file, which file of the
other repository its lines are found in, at the commits you name there.

```console
$ sundgren origin src/reset.py --compare-repo ../upstream --at v1.2,main --format text
src/reset.py: 142 lines
  v1.2 (3f9c0a1e): 89% from lib/calibrate.py (73% distinctive), ahead by 26% of lib/tomography.py (63%)
  main (5b21d7c4): 61% from lib/tomography.py (46% distinctive), ahead by 49% of lib/readout.py (12%)
```

## In VS Code

Open the file and run **sundgren: Show Origin**. Pick the other repository's folder (it is
remembered for this workspace) and the commits to compare with, from its branches and tags. Each
commit is a row: the bar is the share (solid: distinctive lines), the thin bar the next file's
share. A lead of 20 % or more over the next file is marked *clear*, less a *close call*. Click a
row to inspect the file against that commit's best file line by line, edited lines next to the
other file's. **Other** and **Commits** pick again.

## Reading it

- **Share**: how much of the file is found in that file, as for `sundgren contain`: lines
  normalized, the same line or (fuzzy) one at least 90 % similar, comment lines left out in
  Python, Rust and C files. It is containment, not similarity: a file that is two functions
  copied from a much bigger file is 100 % from it.
- **Distinctive**: the share found exactly that is in at most two files there. Imports and other
  boilerplate are in many files, so they don't count here.
- **Ahead by**: the lead over the next file. A high share means little when another file is close
  behind.
- **Several commits**: a file can be mostly from one file at the commit it was copied from, while
  today another file of the other repository wins, after that code was moved or rewritten.

Only files with the same extension are compared. The files are read from the work tree; the other
repository at each commit given with `--at` (repeat it, or separate commits with commas).

`--format json` (the default) prints a `sundgren.origin/0` document
([schema](https://github.com/axean/sundgren/blob/main/schema/origin.schema.json)): per file and
per commit, the best and second file, their shares and the gap. With `--lines`, every line and
where it was found in the best file, as `sundgren contain --lines` lists them. `--fuzzy T` and
`--comments count` work as for `contain`.
