Release Checklist
-----------------

* Run `cargo update` and review dependency updates. Commit updated `Cargo.lock`.
* Run `cargo outdated` and review semver incompatible updates. Unless there is a strong motivation otherwise, review and update every dependency.
* Edit the `Cargo.toml` to set the new version. Run `cargo update -p zinoma` so that the `Cargo.lock` is updated. Commit the changes and create a new tag.
* Wait for CI to finish creating the release. If the release build fails, then delete the tag from GitHub, make fixes, re-tag, delete the release and push.
* Run `cargo publish`.
