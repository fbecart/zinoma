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
- Žinoma 0.15.0

```shell script
$ hyperfine --warmup 2 'gmake sayhello' 'gradle sayHello' 'zinoma say_hello'
Benchmark #1: gmake sayhello
  Time (mean ± σ):       4.5 ms ±   0.4 ms    [User: 2.1 ms, System: 1.4 ms]
  Range (min … max):     3.9 ms …   6.3 ms    423 runs

  Warning: Command took less than 5 ms to complete. Results might be inaccurate.

Benchmark #2: gradle sayHello
  Time (mean ± σ):     574.9 ms ±   7.5 ms    [User: 868.6 ms, System: 91.6 ms]
  Range (min … max):   563.9 ms … 592.6 ms    10 runs

Benchmark #3: zinoma say_hello
  Time (mean ± σ):      27.7 ms ±   1.2 ms    [User: 9.1 ms, System: 5.4 ms]
  Range (min … max):    23.5 ms …  31.6 ms    98 runs

Summary
  'gmake sayhello' ran
    6.22 ± 0.64 times faster than 'zinoma say_hello'
  128.93 ± 12.10 times faster than 'gradle sayHello'
```
