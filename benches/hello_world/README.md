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
- Žinoma 0.14.1

```shell script
$ hyperfine --warmup 2 'gmake sayhello' 'gradle sayHello' 'zinoma say_hello'
Benchmark #1: gmake sayhello
  Time (mean ± σ):       4.7 ms ±   0.7 ms    [User: 2.2 ms, System: 1.5 ms]
  Range (min … max):     3.7 ms …   8.0 ms    392 runs

  Warning: Command took less than 5 ms to complete. Results might be inaccurate.

Benchmark #2: gradle sayHello
  Time (mean ± σ):     572.1 ms ±  10.8 ms    [User: 885.1 ms, System: 95.3 ms]
  Range (min … max):   559.6 ms … 594.4 ms    10 runs

Benchmark #3: zinoma say_hello
  Time (mean ± σ):      28.1 ms ±   1.3 ms    [User: 9.3 ms, System: 5.6 ms]
  Range (min … max):    25.1 ms …  31.0 ms    104 runs

Summary
  'gmake sayhello' ran
    6.04 ± 0.99 times faster than 'zinoma say_hello'
  122.97 ± 19.54 times faster than 'gradle sayHello'
```
