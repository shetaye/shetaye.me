+++
title = 'Thread Equivalence Checking - Part 3'
date = 2024-08-21T17:57:32-05:00
draft = true
+++

Quick recap: there are three phases to our equivalency checker:
1. Collect sequential end states
2. Collect concurrent end states (via interleavings)
3. Compare results from 1 and 2

Now that we can collect and compare end states, we are ready to run
interleavings.

As we defined in part 1, if we have some functions (say `A`, `B`, and `C`), an
*interleaving* between them is one possible way the operating system can
schedule them if each is run as a different thread. An interleaving is
parameterized by the number of context switches it executes; an interleaving of
`n` context switches specifies the starting function, how many instructions to
execute before each context switch, and which function to context switch to for
each context switch.  For example, the following are interleavings with 2
context switches:

```
[A,3,B,9,C]
[C,2,A,3,C]
[B,1,A,1,C]
```

## Executing Interleavings

To execute interleavings we need a way of single stepping through instructions,
or otherwise executing a certain number of instructions before a context switch.
For this we can use the ARM debugging hardware.

### Single Stepping With the ARMv6 Debugger

The ARMv6 architecture supports both external (called halting mode) and internal
(called monitor mode) debugging. External debuggers can stop the entire core and
interrogate system state from there. Internal debuggers, however, raise a debug
exception to handle the debugging event. Debugging events can be anything from
exceptions or faults to watchpoints and breakpoints.

Specifically, the ARMv6 debugger raises a debug exception whenever a software
debug event occurs. Several situations can trigger a software debugging event,
but the one we are interested in is the **Breakpoint Debug Event**. A Breakpoint
Debug Event occurs when
- an instruction is prefetched
- its (pretranslation) address matches the breakpoint value
- the breakpoint conditions match at the time of prefetch
- the instruction is committed for execution
- the processor is not in a privileged mode (**IMPORTANT:** without this
  condition the processor could fall into an infinite loop of exceptions)

If we can trigger Breakpoint Debug Events for each instruction, we can count
instructions and trigger context switches from our debug exception handler.

There's a problem, though. Suppose we set the breakpoint address to an instruction
address `X`. Within the exception handler, we now need to set the breakpoint
address to the next instruction address. However, *how do we know that*? We
could in theory read all relevant processor state then compute the next
instruction address by emulating the ARM control flow logic. However, there is
a *much* easier way to do this. Instead of configuring a breakpoint for hitting
on address *match* the ARMv6 debugging hardware supports a breakpoint for an
address *mismatch*. This means we can configure the breakpoint for some
unreachable address (like `1024`) and break on every instruction we prefetch.

Accomplishing this is relatively straightforward. Configuring the ARMv6 debugger
in monitor mode, enabling a breakpoint, and configuring address mismatch are
all described in the ARM manual, section D3. All configuration happens in
Coprocessor 15.

### Interleaving

Now that we can single step, we are ready to run interleavings. After forking a
thread for each function (I will not cover preemptive threads in this series -
like virtual memory, it deserves its own post), we configure the debugger to
break on mismatch with address `0` (any unreachable address would work). We then
context switch to the first thread in the schedule.

When we hit a debug exception, we have executed another instruction. It is
important here to update any saved registers we associate with the thread, since
we may be context switching in this exception. We then count the instruction and
context switch if enough instructions have executed on this thread.

This is our implementation of the debug exception handler. We use a run queue of
threads and retrieve whichever thread we want to run next in the interleaving
from the run queue. This is guarantees that all threads will run to completion
(sequentially) after the interleaving is done executing. The `ctx_switch_status`
struct is for organized bookkeeping.

```c
// Debug event handler, modified
static void equiv_mismatch_handler(void *data, step_fault_t *s) {
    // Update the current thread's register data
    memcpy(&cur_thread->regs, s->regs, sizeof(regs_t));

    if(ctx_switch_status.do_instr_count) {
      ctx_switch_status.do_instr_count = 0;
      // increment 
      ctx_switch_status.instr_count++;

      // If we have reached the number of expected instructions, context
      // switch
      if (
        ctx_switch_status.instr_count >= 
        schedule->instr_counts[ctx_switch_status.ctx_switch]
      ) {
        ctx_switch_status.ctx_switch++;
        ctx_switch_status.instr_count = 0;

        // context switch
        // equiv_schedule();
        uint32_t tid_idx = ctx_switch_status.ctx_switch;
        eq_th_t* th = retrieve_tid_from_queue(schedule->tids[tid_idx]);
        eq_append(&equiv_runq, cur_thread);
        
        cur_thread = th;
        mismatch_run(&cur_thread->regs);
      }
    }
}
```

Obsere that we also count the number of context switches. If we contex switch
fewer times than we expected, we can use the difference (as described in part 1)
to compute the next interleaving.

## Optimization: Switch Only on Shared Memory Access

So far we have been counting every instruction. This means that we try every
context switch, even those between two instructions that make no memory
accesses. However, this produces many many extra context switches. For example,
if we have the two functions:

```asm
a:
    ldr r2, .GLOBAL
    ldr r1, [r2]
    add r0, r0, r0
    mul r1, r0, r1
    sub r1, r0, r1
    add r0, r0, r1
    str r0, [r2]
    bx lr
b:
    ldr r2, .GLOBAL
    ldr r2, [r2]
    add r0, r0, r0
    mul r1, r0, r1
    sub r1, r0, r1
    add r0, r0, r1
    str r0, [r2]
    bx lr
```

Clearly we only need to consider the orderings of the second loads and first
stores. However, as we have described interleavings up to now, we will consider
every context switch that could possibly occur - even those between two
arithmetic instructions.

The problem is that we are considering context switches on instructions that do
not touch shared memory. Thus, what if we only context switches on instructions
that *do*?

We can make use of our read/write detection from part 2 during interleaving. We
can have a handler that is called by our read/write code for every instruction
that touches global memory (remember, that is what triggers a fault in the first
place). For each of these instructions, we check if they touch some of the
shared memory. If they do, we can conclude that we should count this
instruction and set a flag (in our case, `ctx_switch_status.do_instr_count =
1`). Now, when the *next instruction* is prefetched (and faults), we can look
at the flag and perform switching as necessary. When we come back to this
thread, we will resume at the instruction *after* the shared memory operation.
For example, suppose we have some instructions:

```asm
a:
    ldr <shared>
    str <shared>
b:
    ldr <shared>
    str <shared>
```

Also suppose we have the schedule:

```
[a,1,b]
```

The sequence of events to execute the interleaving is:
1. We start with `a`.
2. We prefetch the `ldr` and a fault occurs
3. Fault is ignored since `do_instr_count == 0` and execution continues.
4. The load executes and a domain fault occurs (this is our read/write tracker).
5. The load is decoded and our handler is called.
6. The handler determines that the instruction touches global memory, so the
   flag is set.
7. We prefetch `str` and a fault occurs
8. Since the flag is set, the fault counts a single instruction execution
9. Interleaving values are checked, and we context switch to `b`

You can see in the above code snippet that we do indeed check the
`do_instr_count` flag and reset it to 0 after each count.

The following is our read/write handler for interleaving:

```c
void ctx_switch_handler(set_t *touched_memory, uint32_t pc) {
    // If we don't have a schedule, give up
    if(!schedule) return;

    // If we are out of context switches, run to completion
    if(ctx_switch_status.ctx_switch >= schedule->n_ctx_switches) return;

    // find intersection of shared memory and touched memory
    set_t *intersection = set_alloc();
    set_intersection(intersection, shared_memory, touched_memory);

    // if the intersection is non-empty
    if (!set_empty(intersection)) {
        if(cur_thread->verbose_p)
          trace("PC %x touched shared memory\n", pc);
        if(schedule->report) {
          schedule->report->pcs[ctx_switch_status.ctx_switch][ctx_switch_status.instr_count] = pc;
        }
        ctx_switch_status.do_instr_count = 1;
    }
    set_free(intersection);
}
```

Now we only count instructions that touch shared memory, vastly decreasing the
search space!

## Locks and Yielding

One final consideration is our interaction with locks backed by atomics. One
potential implementation of a lock is:
```c
#define EQUIV_USER __attribute__((section(".user")))

#include "equiv-threads.h"

#define LOCKED 1
#define UNLOCKED 0

typedef uint32_t vibe_check_t;

EQUIV_USER
static inline uint32_t atomic_compare_and_swap(uint32_t* x, uint32_t old_val, uint32_t new_val) {
  uint32_t result;
  uint32_t status;
  asm volatile (
   "mov %1, #1\n"
   "ldrex %0, [%2]\n"         // Load the value from ptr into result
   "teq %0, %3\n"             // Compare result with old_val
   "strexeq %1, %4, [%2]\n"   // Attempt to store new_val to ptr if equal
   : "=&r" (result), "=&r" (status)
   : "r" (x), "r" (old_val), "r" (new_val)
   : "memory", "cc"
  );
  return status;
}


EQUIV_USER
static inline int vibe_check(vibe_check_t *cur_vibes) {
  if(atomic_compare_and_swap(cur_vibes, UNLOCKED, LOCKED) == 0) return UNLOCKED;
  else return LOCKED;
}

EQUIV_USER
static inline void secure_vibes(vibe_check_t *cur_vibes) {
  while(atomic_compare_and_swap(cur_vibes, UNLOCKED, LOCKED) != 0) {
    sys_equiv_yield();
  }
}

EQUIV_USER
static inline void release_vibes(vibe_check_t *cur_vibes) {
  *cur_vibes = UNLOCKED;
}

// Spin lock functions
static inline void vibe_init(vibe_check_t *cur_vibes) {
    *cur_vibes = UNLOCKED; // 0 indicates that the lock is available
}
```

Note: notice the `EQUIV_USER`? Remember that all checked code must reside in a
special section? We use `.user`. Since we want to check our lock
implementations, they must be placed in the special `.user` section.

When we lock, we keep retrying until we secure the lock. A normal operating
system would prefer to be notified of such a loop. Even better if it is told
what exactly we are waiting for, since it could put the thread to sleep until
after the resource is free and not waste any CPU time spinning in the loop for
even a single time slice. This is normally (like in `pthreads`) accomplished by
exposing a lock API from the OS itself. Another way the notification could occur
is an explicit yield (like the `sys_equiv_yield` above). The yield hints the
operating system to suspend the thread (Unrelated, but still interesting, there
is a Thumb instruction `YIELD` to hint to hardware that the core is in a
spinlock).

*We* need a yield because our checker needs to be told that any further
instructions executed without a context switch would be redundant. For example,
of thread `A` locks a lock and the interleaving switches to `B` which also tries
to lock the lock, we would get stuck in an ever increasing interleaving. The
`sys_equiv_yield` function calls an underlying system call which breaks out of
the checked code and notifies the checker that we are done with this
interleaving and we need to move the prior context switch.

The following is our system call handler:
```c
static int equiv_syscall_handler(regs_t *r) {
    let th = cur_thread;
    assert(th);
    th->regs = *r;  // update the registers

    uart_flush_tx();
    check_sp(th);

    unsigned sysno = r->regs[0];
    switch(sysno) {
    case EQUIV_PUTC: 
        uart_put8(r->regs[1]);
        break;
    case EQUIV_YIELD:
        schedule = NULL;
        ctx_switch_status.yielded = 1;
        if(th->verbose_p)
          trace("Thread %d yielded, running all to completion\n", th->tid);
        
        eq_append(&equiv_runq, cur_thread);
        th = eq_pop(&equiv_runq);
        
        if(th->verbose_p)
          trace("switching from tid=%d,pc=%x to tid=%d,pc=%x,sp=%x\n", 
              cur_thread->tid, 
              cur_thread->regs.regs[REGS_PC],
              th->tid,
              th->regs.regs[REGS_PC],
              th->regs.regs[REGS_SP]);

        cur_thread = th;
        mismatch_run(&cur_thread->regs);
        not_reached();
        break;
    case EQUIV_EXIT: 
        if(schedule) {
          // Run to completion. Disable traps
          if(ctx_switch_status.ctx_switch >= schedule->n_ctx_switches) {
            if(cur_thread->verbose_p)
              trace("Done with schedule\n");
            schedule = NULL;
           // Finished early, disable traps
          } else {
            if(cur_thread->verbose_p)
              trace("Thread %d finished early, running to completion\n", cur_thread->tid);
            schedule = NULL;
          }
        }
        th = eq_pop(&equiv_runq);
        
        if(th && th->verbose_p)
          trace("thread %d next\n", th->tid);

        // if no more threads we are done.
        if(!th) {
            if(th->verbose_p) trace("done with all threads\n");
            switchto(&start_regs);
        }
        // otherwise do the next one.
        cur_thread = th;
        mismatch_run(&cur_thread->regs);
        not_reached();

    default:
        panic("illegal system call: %d\n", sysno);
    }

    not_reached();
}
```

Note: `EQUIV_PUTC` is an example of accessing non-deterministic memory mapped
peripherals (in this case, the `UART` peripheral) by wrapping calls with a
deterministic system call. The system call will always execute succesfully
whether or not the underlying characterprinted.

With yields inserted correctly into atomic spin locks, we can now check many
different data structures and algorithms protected by atomics as well as
traditional mutexes and conditional variables (so long as we correctly emulate
them).

## Parting Words 

As fun as it was to build the Thread Equivalence Checker, there are a number of
improvements that could be made. All of them extend the variety of programs we
can verify.

### POSIX Compliance

We could extend our syscall support to (mostly) POSIX compliance and implement
ELF loading. This would let the checker load arbitrary (statically compiled)
Unix binaries and test them! A good starting point would be anonymous `mmap` for
memory allocation support and `pthreads` for synchronization (we should panic on
forks, though).

### Kernel Integration

Instead of implementing what is basically a full kernel, we could try building
the checker *into* the Linux kernel as either a kernel module or a patch.
Instead of making use of architecture specific single stepping and read-write
set detection, different mechanisms would have to be uesd.

Both kernel integration and POSIX compliance are difficult tasks (arguably more
difficult than writing the tool itself), but they would **significantly** expand
the amount of testable code to a level that would make the tool quite practical
for everyday use.

### Multiprocessor Support

As far as I can tell it is impossible to verify concurrent programs in a
multiprocessor setting with our exhaustive search without certain assumptions.
That is, we can check with multiprocessor support so long as we assume hardware
synchronization primitives are fully functional.
