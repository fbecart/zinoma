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
- Žinoma 0.9.0

```shell script
$ hyperfine --warmup 1 'npm install' 'gradle npmInstall' 'zinoma npm_install'
Benchmark #1: npm install
  Time (mean ± σ):      7.115 s ±  0.408 s    [User: 6.741 s, System: 0.415 s]
  Range (min … max):    6.739 s …  7.701 s    10 runs

Benchmark #2: gradle npmInstall
  Time (mean ± σ):     834.3 ms ±   6.3 ms    [User: 875.3 ms, System: 93.4 ms]
  Range (min … max):   822.2 ms … 844.3 ms    10 runs

Benchmark #3: zinoma npm_install
  Time (mean ± σ):     431.0 ms ±   6.3 ms    [User: 282.7 ms, System: 865.8 ms]
  Range (min … max):   425.1 ms … 445.5 ms    10 runs

Summary
  'zinoma npm_install' ran
    1.94 ± 0.03 times faster than 'gradle npmInstall'
   16.51 ± 0.98 times faster than 'npm install'
```
