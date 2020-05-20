# Node example

This example automates the build flow of [sazzer/docker-test](https://github.com/sazzer/docker-test).

## Setup

Prerequisites:

- [Docker](https://docs.docker.com/get-docker/)
- [yarn](https://classic.yarnpkg.com/en/docs/install)
- [Žinoma](https://github.com/fbecart/zinoma#installation)

```shell script
# Clone this repository including submodules
$ git clone --recurse-submodules git@github.com:fbecart/zinoma.git

# Go to example directory
$ cd zinoma/examples/node
```

### Experimenting with the incremental build

`zinoma e2e_docker_run` will run the full suite of e2e tests in Docker.

It should take about a minute to run the following tasks in parallel:

- build a Docker image of the backend
- build the webapp, then build a Docker image of the webapp
- build a docker image of the e2e tests
- run the e2e tests

This command is incremental. Try to run this command twice, and you'll see:

```shell script
$ zinoma e2e_docker_run
INFO - backend_docker_build - Build skipped (Not Modified)
INFO - e2e_docker_build - Build skipped (Not Modified)
INFO - webapp_install_node_dependencies - Build skipped (Not Modified)
INFO - webapp_build - Build skipped (Not Modified)
INFO - webapp_docker_build - Build skipped (Not Modified)
INFO - e2e_docker_run - Build skipped (Not Modified)
```

This time, the command completed in about one second.
As the sources files haven't been modified, Žinoma was able to understand that the test suite did not need to run again.

Let's introduce a modification to our backend and see what happens:

```shell script
echo "" >> docker-test/backend/index.js
zinoma e2e_docker_run
```

This command should have taken about 30s, which is 50% faster than a full build.

Based on the changes in the file system, Žinoma is able to discriminate which tasks to run, and which tasks to skip.

This time, it was able to skip the build of the webapp and the e2e tests,
but it still had to rebuild the backend and to execute the full test suite.
