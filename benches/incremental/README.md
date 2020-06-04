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
- Žinoma 0.14.0

```shell script
$ hyperfine --warmup 1 'npm install' 'gradle npmInstall' 'zinoma npm_install'
Benchmark #1: npm install
  Time (mean ± σ):      6.900 s ±  0.367 s    [User: 6.540 s, System: 0.394 s]
  Range (min … max):    6.518 s …  7.378 s    10 runs

Benchmark #2: gradle npmInstall
  Time (mean ± σ):     832.5 ms ±  14.5 ms    [User: 871.4 ms, System: 89.1 ms]
  Range (min … max):   813.2 ms … 859.1 ms    10 runs

Benchmark #3: zinoma npm_install
  Time (mean ± σ):     441.8 ms ±  15.0 ms    [User: 296.8 ms, System: 848.2 ms]
  Range (min … max):   418.3 ms … 462.9 ms    10 runs

Summary
  'zinoma npm_install' ran
    1.88 ± 0.07 times faster than 'gradle npmInstall'
   15.62 ± 0.99 times faster than 'npm install'
```
