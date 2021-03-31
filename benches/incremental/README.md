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

- npm 7.5.4
- Gradle 6.8.3 with JVM 1.8.0_242-release (JetBrains s.r.o 25.242-b3-6222593)
- Žinoma 0.19.4

```shell script
$ hyperfine --warmup 1 'npm install' 'gradle npmInstall' 'zinoma npm_install'
Benchmark #1: npm install
  Time (mean ± σ):      3.120 s ±  0.170 s    [User: 2.140 s, System: 0.153 s]
  Range (min … max):    2.964 s …  3.463 s    10 runs

Benchmark #2: gradle npmInstall
  Time (mean ± σ):      1.345 s ±  0.010 s    [User: 1.026 s, System: 0.143 s]
  Range (min … max):    1.335 s …  1.367 s    10 runs

Benchmark #3: zinoma npm_install
  Time (mean ± σ):      1.095 s ±  0.014 s    [User: 731.3 ms, System: 2400.1 ms]
  Range (min … max):    1.077 s …  1.121 s    10 runs

Summary
  'zinoma npm_install' ran
    1.23 ± 0.02 times faster than 'gradle npmInstall'
    2.85 ± 0.16 times faster than 'npm install'
```
