# Pull Requests

## Table of Contents

1. [Before Creating a PR](#before-creating-a-pr)
2. [Creating a PR](#creating-a-pr)
	* [PR Title](#pr-title)
	* [PR Summary](#pr-summary)
	* [Formatting, Linting, Etc.](#formatting-linting-etc)
	* [Tests](#tests)

## Before Creating a PR

Before you create a PR, we ask that you do the following:

1. **If fixing a bug, make an issue first**: The issue tracker should have a searchable history of
bugs reports.

2. **If adding a feature, make an issue or discussion first**: Features should be discussed prior to implementation.

## Creating a PR

There are two additional documents covering more substantial contributions:

* [Creating a new file type](NEW_FILE.md)
* [Creating a new tag type](NEW_TAG.md)

The rest of this document covers general PR procedures.

### PR Title

See [Issue Title](ISSUES.md#issue-title).

### PR Summary

Please provide a description of the change(s) made, unless they can be easily inferred from the title.
This should only provide a brief overview of the implementation details, with relevant links to specifications,
issues, etc.

Also be sure to mention the issue associated with the PR like so: "closes #10".

### Formatting, Linting, Etc.

Lofty uses the traditional tools `rustfmt`, `clippy`, and `rustdoc` to keep a consistent style and maintain
correctness.

Prior to finalizing a PR, it is a good idea to run `cargo fmt`, `cargo clippy`, and `cargo doc` to ensure
there are no errors. These commands are also run in CI using the latest stable Rust.

### Tests

It is incredibly important that a PR provides tests for any behavioral changes.

* When fixing a bug, create a test that mirrors the reproducer provided in the issue.
* When adding a feature, create tests for any new additions made where sensible
