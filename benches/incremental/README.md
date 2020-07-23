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
- Žinoma 0.18.2

```shell script
$ hyperfine --warmup 1 'npm install' 'gradle npmInstall' 'zinoma npm_install'
Benchmark #1: npm install
  Time (mean ± σ):      7.417 s ±  0.359 s    [User: 6.868 s, System: 0.406 s]
  Range (min … max):    6.748 s …  7.684 s    10 runs

Benchmark #2: gradle npmInstall
  Time (mean ± σ):     860.1 ms ±  10.0 ms    [User: 890.8 ms, System: 92.6 ms]
  Range (min … max):   848.4 ms … 876.7 ms    10 runs

Benchmark #3: zinoma npm_install
  Time (mean ± σ):     525.0 ms ±   5.6 ms    [User: 273.0 ms, System: 540.1 ms]
  Range (min … max):   515.5 ms … 532.7 ms    10 runs

Summary
  'zinoma npm_install' ran
    1.64 ± 0.03 times faster than 'gradle npmInstall'
   14.13 ± 0.70 times faster than 'npm install'
```
