Release Checklist
-----------------

* Run `cargo update` and review dependency updates. Commit updated `Cargo.lock`.
* Run `cargo outdated` and review semver incompatible updates. Unless there is a strong motivation otherwise, review and update every dependency.
* Edit the `Cargo.toml` to set the new version. Run `cargo update -p zinoma` so that the `Cargo.lock` is updated.
* In `CHANGELOG.md`, update the release title with its version & the current date.
* Commit the changes, create a new tag and push it along with the master branch.
* Wait for CI to finish creating the release. If the release build fails, then delete the tag from GitHub, make fixes, re-tag, delete the release and push.
* Run `cargo publish`.
* Run `ci/sha256-releases.sh {VERSION} >> HomebrewFormula/zinoma.rb`. Then edit `HomebrewFormula/zinoma.rb` to update the version number and sha256 hashes. Remove extraneous stuff added by `ci/sha256-releases`. Commit changes.
