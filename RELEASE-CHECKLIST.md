# Release Checklist

- [ ] Run `cargo update` and review dependency updates. Commit updated `Cargo.lock`.
- [ ] Edit the `Cargo.toml` to set the new version. Run `cargo update -p zinoma` so that the `Cargo.lock` is updated.
- [ ] In `CHANGELOG.md`, add a release title with its version & the current date.
- [ ] Commit the changes, create a new tag and push it along with the master branch.
- [ ] Wait for CI to finish creating the release. If the release build fails, then delete the tag and the release from GitHub.
