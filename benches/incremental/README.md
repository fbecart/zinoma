# Incremental build benchmark

This benchmark evaluates the performances of Žinoma when dealing with incremental build flows dealing with lots of files.

We'll use an npm project as a way to generate a large output directory (here, `node_modules`).
Then, we'll compare three tools to generate this directory in an incremental way:

- npm
- Gradle
- Žinoma

The benchmark uses Hyperfine to compare the three commands.

## On my machine (MacBook Pro from 2015, Mac OS X 10.14.6 x86_64)

These results would probably be hard to reproduce, even on my own machine. Please take them with a grain of salt!

Node was installed with nvm. Gradle and Žinoma were installed with brew:

```shell script
brew install gradle fbecart/tap/zinoma
```

Versions:

- npm 6.14.5
- Gradle 6.4.1 with JVM 1.8.0_101 (Oracle Corporation 25.101-b13)
- Žinoma 0.8.1

```shell script
$ hyperfine 'npm install' 'gradle npmInstall' 'zinoma npm_install'
Benchmark #1: npm install
  Time (mean ± σ):      7.045 s ±  0.401 s    [User: 6.693 s, System: 0.408 s]
  Range (min … max):    6.646 s …  7.594 s    10 runs

Benchmark #2: gradle npmInstall
  Time (mean ± σ):     836.8 ms ±  13.7 ms    [User: 883.6 ms, System: 92.9 ms]
  Range (min … max):   823.3 ms … 867.2 ms    10 runs

Benchmark #3: zinoma npm_install
  Time (mean ± σ):     445.6 ms ±   4.8 ms    [User: 300.8 ms, System: 857.3 ms]
  Range (min … max):   440.2 ms … 456.9 ms    10 runs

Summary
  'zinoma npm_install' ran
    1.88 ± 0.04 times faster than 'gradle npmInstall'
   15.81 ± 0.92 times faster than 'npm install'
```
