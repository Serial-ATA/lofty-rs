# Issues

## Table of Contents

1. [Before Reporting an Issue](#before-reporting-an-issue)
2. [Reporting Bugs](#reporting-bugs)
    * [Issue Title](#issue-title)
    * [Issue Summary](#issue-summary)
    * [Reproducer](#reproducer)
    * [Expected Behavior](#expected-behavior)
    * [Actual Behavior](#actual-behavior)
    * [Assets](#assets)
      * [A Note on Copyright](#a-note-on-copyright)
3. [Feature Requests](#feature-requests)
    * [Miscellaneous Feature Requests](#miscellaneous-feature-requests)
    * [API Feature Requests](#api-feature-requests)

## Before Reporting an Issue

Before you report an issue, we ask that you do the following:

1. **Search for existing issues**: Ensure that the issue you encountered has not already been reported by searching the issue tracker.
If you find a related issue, you can contribute by adding any additional information or subscribing to receive updates.

2. **Update to the latest version**: Make sure you are using the most recent version of the project.
Sometimes, issues might have already been resolved in the latest release.

## Reporting Bugs

When reporting an issue, please provide as much relevant information as possible.
This will help us understand and address the problem efficiently.

### Issue Title

Choose a descriptive and concise title that summarizes the problem you encountered,
and the area it occurred in. A good title provides a clear idea of the issue at a glance.

An example of a good title: "ID3v2: Tag padding is treated as a frame"

### Issue Summary

In the issue summary, provide a detailed explanation of the problem you are facing.

Be specific and avoid ambiguity. Include information such as:

* Any error/panic messages related to the problem.
* Any relevant configurations or settings you have modified.

### Reproducer

Include a set of clear and concise steps that can reproduce the issue you encountered.
These steps should be detailed enough for others to follow and observe the same problem.
Providing a minimal, standalone code example or a sample input can be immensely helpful.

Unless necessary, please do not provide code with unnecessary context or inaccessible custom types.

### Expected Behavior

Describe what you expected to happen when you encountered the issue.
This information helps us understand the desired outcome and compare it to the actual behavior.

### Actual Behavior

Explain what actually happened when you encountered the issue.
Include any observed discrepancies or unexpected behavior.
This information will aid us in diagnosing the problem accurately.

### Assets

If applicable, provide any additional assets that might be relevant to the issue.
This can include files you have attempted to read, related discussions,
or links to relevant resources (specs, issues in other projects, etc.).

#### A Note on Copyright

Please do not upload copyrighted content to your issue. If there is an issue that you can only produce
with an asset that happens to be copyrighted, you can email it to `serial (AT) [domain on my profile]`.

If you choose to email an asset, please be sure to state this in the issue.

## Feature Requests

There are two ways to create a feature request:

* [Using the "[MISC] Feature Request" issue template](#miscellaneous-feature-requests)
* [Using the "[API] Feature Request" issue template](#api-feature-requests)

### Miscellaneous Feature Requests

The "[MISC] Feature Request" issue template is for feature requests that either:

* Have no public API
* Are not fully fleshed out
* Implementation details are not too important

### API Feature Requests

The "[API] Feature Request" issue template is for feature requests that have a specific design.

If you have an feature with an exact idea of how it should be used, be sure to use this template.

The "API design" form should only serve as a design, not an implementation.

For example, do this:

```rust
pub struct MyTag {
    some_field: ty,
    some_other_field: ty,
    // ... omit other fields if extending an existing type
}

impl MyTag {
    pub fn foo(&self) -> String;
}
```
