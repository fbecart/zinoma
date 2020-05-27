# Hello World benchmark

This benchmark evaluates the execution of a trivial task (printing "Hello, World!") by the following tools:

- Make
- Gradle
- Žinoma

It uses Hyperfine to compare the three commands.

## On my machine (MacBook Pro from 2015, Mac OS X 10.14.6 x86_64)

These results would probably be hard to reproduce, even on my own machine. Please take them with a grain of salt!

Those tools were installed with brew:

```shell script
brew install make gradle fbecart/tap/zinoma
```

Versions:

- GNU Make 4.3
- Gradle 6.4.1 with JVM 1.8.0_101 (Oracle Corporation 25.101-b13)
- Žinoma 0.9.0

```shell script
$ hyperfine --warmup 2 'gmake sayhello' 'gradle sayHello' 'zinoma say_hello'
Benchmark #1: gmake sayhello
  Time (mean ± σ):       4.6 ms ±   0.5 ms    [User: 2.2 ms, System: 1.5 ms]
  Range (min … max):     3.8 ms …   7.4 ms    420 runs

  Warning: Command took less than 5 ms to complete. Results might be inaccurate.

Benchmark #2: gradle sayHello
  Time (mean ± σ):     553.7 ms ±   4.1 ms    [User: 876.6 ms, System: 92.3 ms]
  Range (min … max):   545.9 ms … 560.3 ms    10 runs

Benchmark #3: zinoma say_hello
  Time (mean ± σ):      39.9 ms ±   2.0 ms    [User: 9.5 ms, System: 7.7 ms]
  Range (min … max):    36.4 ms …  45.2 ms    70 runs

Summary
  'gmake sayhello' ran
    8.70 ± 1.00 times faster than 'zinoma say_hello'
  120.58 ± 12.58 times faster than 'gradle sayHello'
```
