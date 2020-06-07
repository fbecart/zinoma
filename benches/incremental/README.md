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
- Žinoma 0.14.1

```shell script
$ hyperfine --warmup 1 'npm install' 'gradle npmInstall' 'zinoma npm_install'
Benchmark #1: npm install
  Time (mean ± σ):      7.081 s ±  0.376 s    [User: 6.725 s, System: 0.415 s]
  Range (min … max):    6.635 s …  7.603 s    10 runs

Benchmark #2: gradle npmInstall
  Time (mean ± σ):     857.7 ms ±  22.0 ms    [User: 887.3 ms, System: 93.4 ms]
  Range (min … max):   832.9 ms … 911.0 ms    10 runs

Benchmark #3: zinoma npm_install
  Time (mean ± σ):     418.3 ms ±   2.6 ms    [User: 277.5 ms, System: 848.9 ms]
  Range (min … max):   414.4 ms … 422.7 ms    10 runs

Summary
  'zinoma npm_install' ran
    2.05 ± 0.05 times faster than 'gradle npmInstall'
   16.93 ± 0.90 times faster than 'npm install'
```
