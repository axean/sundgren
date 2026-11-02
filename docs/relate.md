---
orphan: true
---

# Relate

> [!NOTE]
> A prototype.

When a repository was copied (an import, a fork, a mirror) and both went on, git can still tell
where they split and how they were synced since, if you have a clone of each. Here `copy` was
cloned from `upstream` after u2, merged it in twice (m1, m2), merged u6 on a side branch only, and
upstream went on to u7:

```console
$ sundgren relate ../upstream
copy (/work/copy): 11 commits, 9 on main (54cbe096)
upstream (/work/upstream): 7 commits, 7 on main (1d16b207)
shared: 6 commits, 1 root
copy's main left upstream's main at 7b7dd623 2026-01-02 12:00  u2
  first own 17db64e8 2026-01-05 12:00  a1
  the copy: made between 2026-01-02 12:00 and 2026-01-03 12:00
upstream's main left copy's main at 85ee6d5f 2026-01-07 12:00  u5
  first own c0908869 2026-01-10 12:00  u6
last sync (merge base): 85ee6d5f 2026-01-07 12:00  u5
newest shared: c0908869 2026-01-10 12:00  u6
  not on copy's main; reaches it through side, origin/main
merges into copy's main from upstream's main:
  ed233a96 2026-01-06 12:00  +2 up to 62e0cb16  m1
  54cbe096 2026-01-09 12:00  +1 up to 85ee6d5f  m2
after the copy, 3 commits are on both bases: 0 first on copy, 3 first on upstream, 0 at the same time
named in copy's messages:
  team/copy     1 merged  2026-01-06 .. 2026-01-06  !1..!1
  links: example.org 1
named in upstream's messages:
  links: example.org 1
```

Each side has a base, its default branch unless given (`--base`, `--other-base`).

- **Copy point**, per side: walking the base's first-parent chain, the first parent of the oldest
  commit the other base doesn't have.
- **The copy** is the side with the older copy point. **Made between**: from its copy point to the
  earlier of the other base's next first-parent commit and its first own commit. Git dates only,
  so a copy made later without new commits on either side looks the same.
- **Last sync**: the merge base of the two bases.
- **Newest shared**: the newest commit both repositories have anywhere, and which refs bring it to
  a side whose base doesn't have it.
- **Merges**: first-parent merges on one base that brought in commits of the other's base, with
  how many and the newest of them.
- **First on**: for the commits both bases got after the copy, which base had them first, by the
  first-parent commit that brought them in.
- **Named in messages**, per repository, over all its commits: the repository paths of GitLab's
  `See merge request PATH!N` lines, with how many, the first and last date and the numbers; and
  the hosts of `http(s)://` links. A copy made by an import often names where it came from.

## Without touching either clone

Both are read as one bare repository in `$XDG_CACHE_HOME/sundgren/join` (default `~/.cache`), which
borrows their objects (`objects/info/alternates`) and copies their refs on every run: this one's as
they are, the other's as a remote named after its folder (`--name` to change it). Nothing is
written to either clone. The cache holds only refs; delete it any time.

`--format json` prints a `sundgren.relate/0` document
([schema](https://github.com/axean/sundgren/blob/main/schema/relate.schema.json)), with the commits
above also as a list of marks.
