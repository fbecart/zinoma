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
- Žinoma 0.17.0

```shell script
$ hyperfine --warmup 1 'npm install' 'gradle npmInstall' 'zinoma npm_install'
Benchmark #1: npm install
  Time (mean ± σ):      7.420 s ±  0.097 s    [User: 6.649 s, System: 0.389 s]
  Range (min … max):    7.294 s …  7.570 s    10 runs

Benchmark #2: gradle npmInstall
  Time (mean ± σ):     836.2 ms ±  15.3 ms    [User: 871.6 ms, System: 88.5 ms]
  Range (min … max):   812.5 ms … 869.0 ms    10 runs

Benchmark #3: zinoma npm_install
  Time (mean ± σ):     510.7 ms ±   6.7 ms    [User: 268.1 ms, System: 523.6 ms]
  Range (min … max):   500.8 ms … 525.4 ms    10 runs

Summary
  'zinoma npm_install' ran
    1.64 ± 0.04 times faster than 'gradle npmInstall'
   14.53 ± 0.27 times faster than 'npm install'
```
