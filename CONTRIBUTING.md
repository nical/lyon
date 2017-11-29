# Contributing to lyon

Lyon welcomes contribution from everyone. Here are the guidelines if you are
thinking of helping us:

## Contributions

Contributions to lyon should be made in the form of GitHub pull requests.
Each pull request will be reviewed by a core contributor (someone with
permission to land patches) and either landed in the main tree or
given feedback for changes that would be required.
All contributions should follow this format, even those from core contributors.

Should you wish to work on an issue, please claim it first by commenting on
the GitHub issue that you want to work on it. This is to prevent duplicated
efforts from contributors on the same issue.

After your first contribution is merged, your name/handle will be added to the [list
of contributors](https://github.com/nical/lyon/wiki/Contributors). If your name
was forgotten or should you wish to not appear in the list, contact Nical (nical@fastmail.com).

## Getting started

Have a look at the [issues with the "help wanted' label](https://github.com/nical/lyon/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22) to find
good tasks to start with. Help is of course welcome with all issues filed, but
the "help wanted" ones are good places to start because they don't require a lot
of prior knowledge about the project. Most of these issues are labelled
[easy](https://github.com/nical/lyon/issues?q=is%3Aissue+is%3Aopen+label%3Aeasy),
[medium](https://github.com/nical/lyon/issues?q=is%3Aissue+is%3Aopen+label%3Amedium) or
[hard](https://github.com/nical/lyon/issues?q=is%3Aissue+is%3Aopen+label%3Ahard) depending
on their estimated difficulty.

If you are interested in working on the tessellators, the wiki has some information
about the algortithms that may be useful to you

 - [fill tessellator](https://github.com/nical/lyon/wiki/Tessellator)
 - [stroke tessellator](https://github.com/nical/lyon/wiki/Stroke-tessellation)

## Pull Request Checklist

- Branch from the master branch and, if needed, rebase to the current master
  branch before submitting your pull request. If it doesn't merge cleanly with
  master you may be asked to rebase your changes.

- Commits should be as small as possible, while ensuring that each commit is
  correct independently (i.e., each commit should compile and pass tests).

- If your patch is not getting reviewed or you need a specific person to review
  it, you can @-reply a reviewer asking for a review in the pull request or a
  comment.

- Whenever applicable, add tests relevant to the fixed bug or new feature.

For specific git instructions, see [GitHub workflow 101](https://github.com/servo/servo/wiki/Github-workflow).

## Testing

To run all tests from all of the lyon crates, run `cargo test --all` from the root of the repository.

## Conduct

In all lyon-related forums, we follow the [Rust Code of Conduct](http://www.rust-lang.org/conduct.html).
For escalation or moderation issues, please contact Nical (nical@fastmail.com) instead of the Rust moderation team.

## Communication

[Gitter](https://gitter.im/lyon-rs/Lobby) and the [github issues](https://github.com/nical/lyon/issues) are good places to ask questions and more generally talk about about lyon. Some of the lyon contributors also frequent the `#rust` and `#rust-gamedev` channels on [`irc.mozilla.org`](https://wiki.mozilla.org/IRC).

## License

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be licensed dual MIT/Apache 2, or Mozilla Public License 2.0, without any additional terms or conditions.

We reserve the right to publish future versions of this project under the Mozilla Public License 2.0 (MPL2) instead of the three-licenses scheme described above, including all contributions.
If such a change occurs, it will not affect versions of the project prior to the application of the license change.

### In other words...

At this time we are not entirely certain which of MPL2 or dual MIT/Apache2 is the best licensing scheme for this project. Both approaches are very permissive and we want to keep the door open to changing to MPL2 in the future.
This project will remain usable under the MIT/Apache2 license for the foreseeable future. However, by contributing to this project you accept that your contributions may be re-licensed under the Mozilla Public License 2.0 without the MIT and Apache2 options.
