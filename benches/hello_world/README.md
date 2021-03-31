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
- Gradle 6.8.3 with JVM 1.8.0_242-release (JetBrains s.r.o 25.242-b3-6222593)
- Žinoma 0.19.4

```shell script
$ hyperfine --warmup 2 'gmake sayhello' 'gradle sayHello' 'zinoma say_hello'
Benchmark #1: gmake sayhello
  Time (mean ± σ):       5.8 ms ±   0.5 ms    [User: 2.4 ms, System: 2.1 ms]
  Range (min … max):     5.1 ms …   7.5 ms    359 runs

Benchmark #2: gradle sayHello
  Time (mean ± σ):     691.4 ms ±   9.0 ms    [User: 1.025 s, System: 0.142 s]
  Range (min … max):   674.3 ms … 701.0 ms    10 runs

Benchmark #3: zinoma say_hello
  Time (mean ± σ):      12.4 ms ±   0.7 ms    [User: 7.1 ms, System: 6.6 ms]
  Range (min … max):    11.0 ms …  16.7 ms    198 runs

Summary
  'gmake sayhello' ran
    2.14 ± 0.22 times faster than 'zinoma say_hello'
  118.90 ± 9.96 times faster than 'gradle sayHello'
```
