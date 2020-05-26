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
- Žinoma 0.8.0

```shell script
$ hyperfine 'gmake sayhello' 'gradle sayHello' 'zinoma say_hello'
Benchmark #1: gmake sayhello
  Time (mean ± σ):       4.4 ms ±   0.4 ms    [User: 2.1 ms, System: 1.4 ms]
  Range (min … max):     3.8 ms …   6.0 ms    402 runs

  Warning: Command took less than 5 ms to complete. Results might be inaccurate.

Benchmark #2: gradle sayHello
  Time (mean ± σ):     554.4 ms ±   4.6 ms    [User: 859.3 ms, System: 87.9 ms]
  Range (min … max):   549.3 ms … 565.8 ms    10 runs

Benchmark #3: zinoma say_hello
  Time (mean ± σ):      40.6 ms ±   2.0 ms    [User: 9.3 ms, System: 7.3 ms]
  Range (min … max):    36.6 ms …  45.2 ms    66 runs

Summary
  'gmake sayhello' ran
    9.16 ± 0.97 times faster than 'zinoma say_hello'
  125.00 ± 11.85 times faster than 'gradle sayHello'
```
