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
- Žinoma 0.14.0

```shell script
$ hyperfine --warmup 2 'gmake sayhello' 'gradle sayHello' 'zinoma say_hello'
Benchmark #1: gmake sayhello
  Time (mean ± σ):       4.5 ms ±   0.5 ms    [User: 2.1 ms, System: 1.4 ms]
  Range (min … max):     3.8 ms …   6.5 ms    435 runs

  Warning: Command took less than 5 ms to complete. Results might be inaccurate.
  Warning: Statistical outliers were detected. Consider re-running this benchmark on a quiet PC without any interferences from other programs. It might help to use the '--warmup' or '--prepare' options.

Benchmark #2: gradle sayHello
  Time (mean ± σ):     554.1 ms ±   6.7 ms    [User: 861.9 ms, System: 88.7 ms]
  Range (min … max):   541.3 ms … 564.3 ms    10 runs

Benchmark #3: zinoma say_hello
  Time (mean ± σ):      42.1 ms ±   2.3 ms    [User: 9.6 ms, System: 7.5 ms]
  Range (min … max):    38.3 ms …  47.5 ms    66 runs

Summary
  'gmake sayhello' ran
    9.34 ± 1.06 times faster than 'zinoma say_hello'
  122.82 ± 12.40 times faster than 'gradle sayHello'
```
