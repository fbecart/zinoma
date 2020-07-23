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
- Žinoma 0.18.2

```shell script
$ hyperfine --warmup 2 'gmake sayhello' 'gradle sayHello' 'zinoma say_hello'
Benchmark #1: gmake sayhello
  Time (mean ± σ):       4.6 ms ±   0.8 ms    [User: 2.2 ms, System: 1.5 ms]
  Range (min … max):     3.8 ms …   7.9 ms    460 runs

  Warning: Command took less than 5 ms to complete. Results might be inaccurate.

Benchmark #2: gradle sayHello
  Time (mean ± σ):     571.5 ms ±   7.0 ms    [User: 890.6 ms, System: 92.6 ms]
  Range (min … max):   558.6 ms … 582.9 ms    10 runs

Benchmark #3: zinoma say_hello
  Time (mean ± σ):      17.9 ms ±   1.8 ms    [User: 9.5 ms, System: 6.4 ms]
  Range (min … max):    13.0 ms …  22.7 ms    154 runs

Summary
  'gmake sayhello' ran
    3.88 ± 0.75 times faster than 'zinoma say_hello'
  124.08 ± 20.41 times faster than 'gradle sayHello'
```
