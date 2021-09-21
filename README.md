# 𓇚 Ka - A file-based VCS
Ka keeps it's repositories up-to-date with simple change operations, while at the same time preventing conflicts and keeping ahold of the entire history of every file.

#### Idea
We can keep track of files in a directory by keeping a changeset of all bytes changed.
Compared to other VSC solutions we can offer less flexibility in favor of a better workflow for other tools: 
* No possibility to stage files: All files in a directory with a `.ka` directory inside of it, from now on named "working directory", automatically count as tracked after their first update.
* No presumptions over how to sync remote and local Ka repositories, letting syncing be handled by other components. 
* We split changes by file, therefore getting rid of complex multi-file change-sets, essentially version controlling every file individually. 
* We don't try to deduce complex changes like renames, the only two operations done on a file shall be *Update*, *Delete* and an implicit add as the first update.
* No true support of branching, all changes are linear. If a Git repository is a tree, then a Ka repository is a flower garden.
* No metadata apart from a the datetime of a change.

#### Structure
The state of a Ka repository is kept inside a directory named `.ka` inside the working directory, similar to `.git`. It is the only directory which is excluded by tracking.

Directly inside of `.ka` there shall be an `index` file.
This file keeps every single change which happened to each file in chronological order, without the actual content of the change, using the index assigned to every change, and a `Cursor` pointing to such one index, usually the top-most one.

Continuing with the contents of `.ka`, there lies another directory named `files`, which from it's structure is a mirror of the tracked directory itself. However, it also includes deleted and moved files. These files keep the history of the changes done to their respective mirror files in the tracked directory.

#### API
Ka shall be used either through a library or it's CLI, which is very limited in it's ability.
Both usages expose a limited API composed out of these functions:
* **Create** - Creates a Ka repository from the working directory and updates it. Using it on an existing repository essentially flattens it's history.
* **Update** - The simplest mutating operation done on a Ka repository. Applies all changes to all files. You are not allowed to exclude files.
* **Shift** - Shifts a `Cursor` of a file to the given index of a change. Note that *Shift* is non-destructive by itself, until *Update* is used, which removes all changes made after the change where the `Cursor` is placed forever.
