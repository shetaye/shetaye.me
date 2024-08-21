+++
title = 'Thread Equivalence Checking - Part 1'
date = 2024-07-08T20:20:45-07:00
draft = false
+++

Concurrent programs are complicated to write and even more complicated to get
right. To deal with this, my friend
[Aaryan](https://www.linkedin.com/in/aaryan-singhal-151aa91a6) and I wrote a
tool for verifying the correctness of multithreaded programs.

I'll present this project as a series of blog posts. In the first, I want to
give a motivation and a high-level overview of the project.

## Equivalency 

Languages like C define an *abstract machine* that is the target of the
language. This abstract machine is sequential, and executes statements in their
entirety before moving on to the next.  Real machines are much more nuanced than
this, hence the *abstract* in abstract machine, but it's a useful and portable
approximation. Since sequential execution is by design, we can begin by saying
that sequential execution is the "correct" way for a progrma to execute. That
is, for some functions, we can say that any final result (i.e. observable state)
produced by a sequential order of the functions is correct.

For example, consider the following program (compiled without heavy optimization):

```c
int global = 0;

void a() {
    int local = global;
    local += 2;
    global = local;
}

void b() {
    int local = global;
    local *= 2;
    global = local;
}
```

There are only two ways of running the functions sequentially: running `a` then
`b` or `b` then `a`. The former leaves 4 in `global`, and the latter leaves 2 in
`global`. Thus, the two valid end states are `global = 2` and `global = 4`.

```c
include <stdio.h>

void main() {
    a(); b();
    printf("global = %d\n", global);
    global = 0;
    b(); a();
    printf("global = %d\n", global);
}
```

However, if we allow `a` and `b` to be run *concurrently*, the operating
system can schedule `a` and `b` however it pleases. This introduces several more
possibilities.

For example, `a` can be run up through loading global, then the OS switches to
`b`, which loads up through loading global, then `a` is run to completion, then
`b` is run to completion. The result is `global = 0`. This is not a possible
state if we stick with the abstract machine.

The goal of the thread equivalence checker is to verify, with complete certainty
(see appendix), that every way we can run `a` and `b` concurrently produce a
resulting state that is identical to the final state of a sequential execution
of `a` and `b`. That is to say, they are part of the same **equivalence** set
and our program is sequentially equivalent.

One way to ensure concurrent execution of `a` and `b` end in sequential final
states is by using synchronization primitives. Synchronization primitives
maintain the illusion of an abstract machine with the performance benefits of
multithreading. 

For example, with a toy mutex implementation, we can fix our functions.

```c
int global = 0;
mutex_t mut;

void a() {
    lock(mut);
    int local = global;
    local += 2;
    global = local;
    unlock(mut);
}

void b() {
    lock(mut);
    int local = global;
    local *= 2;
    global = local;
    unlock(mut);
}
```

Now if `a` is interrupted by `b`, `b` will not be able to modify or even read
`global` since its call to `lock(mut)` would hang (and probably cause it to be
interrupted by another thread).  The above implementation of `a` and `b` is now
equivalent to a sequential one!

## Approach 

Ok, so how do we go about proving (or disproving) sequential equivalency for
functions?

More generally, we want to check if any number of functions are sequentially
equivalent. To do so, we need to 
1. Collect valid end states by running functions sequentially
2. Collect all possible end states by running functions concurrently
3. Compare the results from (1) and (2) to ensure they are equal

### Sequential End States

To tackle 1, we can compute all permutations of the given functions (for the
above it's `a,b` and `b,a`), run each sequence, and collect the end states.

### Concurrent End States

For 2, we need to enumerate all ways the functions can interrupt eachother. To
do so, we need to define what it means to be interrupted.  

For simplicity's sake (and to make the problem at all tractable), we assume that
the functions are all running on a single core (see Appendix). This means that
when the thread running `a` is interrupted by the thread running `b`, the OS
context switches from the first to the second. The execution state (address of
next or last instruction, stack pointer, registers, etc.) of `a` is saved
somewhere (usually on the stack) and the execution state of `b` is restored.

For our purposes, what matters is that for *a single context switch* from `a` to
`b`, there are some number of instructions executed on `a`, then all of `b` is
executed, then the remainder of `a` is executed. Let's represent that as a
list: `[x,b]` where we execute `x` instructions on `a` then switch to `b` and
run it to completion, then switch to `a` and run it to completion. We could have
started on `b`, so we should add a term at the start to state which thread we
start on: `[a,x,b]`. 

More generally, the format goes: 
```
[start thread, # instructions 0, thread 0, # instructions 1, ..., # instructions n]
```
where we start on the `start thread`, run for `# instructions 0`, switch to `ctx
switch 0`, and so on. We call this an **interleaving** of threads with `n`
context switches.

Now, let's enumerate all interleavings. Let's focus on enumerating the
instruction counts first. First, run the interleaving. Now, add 1 to the final
instruction count. Run again. You could keep doing this, but how do you know
when to stop? Well, let's return to the prior example, but let's compile it
(with optimization):

```asm
a():
        ldr     r2, .L3
        ldr     r3, [r2]
        add     r3, r3, #2
        str     r3, [r2]
        bx      lr
.L3:
        .word   .LANCHOR0
b():
        ldr     r2, .L6
        ldr     r3, [r2]
        lsl     r3, r3, #1
        str     r3, [r2]
        bx      lr
.L6:
        .word   .LANCHOR0
```

Suppose we only want one context switch. We could start with the following
interleaving: `[a, 1, b]`. Then we increment to `[a, 2, b]`, and so on. However,
after `[a, 5, b]`, we've run out of instructions in `a` to execute. Further
interleavings with the same prefix (such as `[a, 100, b]`) are meaningless and
redundant. At this point we're done (aside from enumerating threads).

Furthermore, we can apply this to any number of context switches. If we have `n`
context switches and we run an interleaving that context switches `m` times, we
need to increment the `(n - m)th` instruction count from the end.

Ok, so what happens after enumerating all the instruction counts? We need to
enumerate all destination threads (plus the starting thread). This is simple
enough: form all permutations with replacement of `n + 1` threads such that no
two adjacent threads are the same. This reads like a LeetCode problem so the
algorithm is left as an exercise to the reader.

### Equivalency Check

After enumerating all possible interleavings, running each, and collecting their
end states, we check if each resulted in an end state that is in the set of
sequential end states. If so, the functions are correct. If not, there is a bug.

## Stay Tuned

This post covered the idea behind our thread checker. However, it left the
implementation blank: how do we run an interleaving? how do we collect and
compare end states? In the next posts we'll use Arm debugging features and the
MMU to do all of the above!

## Appendix

Assumptions:
1. A uniprocessor system: all instructions execute atomically and cannot be
   interrupted
2. Deterministic functions: given a specific interleaving, functions will
   always execute the same instructions
