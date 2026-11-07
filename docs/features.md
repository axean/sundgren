---
orphan: true
---

# Features

What sundgren does. History and Containment have a live demo of the VS Code panel, on a made-up
repository; the others show the command line.

[History](history.md)
: A map of branches around the tip of a base branch: where each one leaves, merges and stands
  against its remote, routed like a circuit board. **sundgren: Show History** in VS Code,
  `sundgren graph` on the command line.

[Containment](containment.md)
: How much of each branch's work is already in another branch, and the other way round, by
  content rather than ancestry. It finds extractions, rebased copies and overlapping work.
  **sundgren: Show Containment** in VS Code, `sundgren contain` on the command line.

[Relate repositories](relate-repos.md)
: Two clones of related repositories, a copy and its upstream: where they split, when the copy
  was made, and how they were synced since. `sundgren relate-repos` on the command line.

[Origin](origin.md)
: Which file of another repository a file's lines come from, at the commits you name there.
  **sundgren: Show Origin** in VS Code, `sundgren origin` on the command line.
