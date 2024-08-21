+++
title = 'Thread Equivalence Checking - Part 2'
date = 2024-08-19T16:57:26+02:00
draft = false
+++

Quick recap: there are three phases to our equivalency checker:
1. Collect sequential end states
2. Collect concurrent end states (via interleavings)
3. Compare results from 1 and 2

First, let's lay some foundations. We need to answer a couple important
questions: what *are* end states? how do we collect them? how do we compare
them?

## Defining an End State

Suppose we go back to our two functions from before:
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

After executing `a` and `b`, the only observable side effects (assuming
deterministic execution without external side effects e.g. accessing system
registers) will be in global memory. Any stack memory will no longer be
observable (if you follow the rules) after the functions returned and their
stack space was cleared. So our first definition of an end state for `a` and `b`
is all global memory.

We can also see that if there was a second global, say `global2`, that is not
accessed by either function, its value would be irrelevant when checking the
final state of `a` and `b` (remember, we want to keep track of *observable* side
effects). We can shrink our definition to all global memory that is accessed by
any two of the functions we are checking.

What if there is a global value that is only *read* by our functions?
Its value would *also* be irrelevant to final state, since we could assert from
the start that its value would remain identical to its starting value after any
execution of `a` and `b`.

Finally, what if only one function accesses global memory? Even with reads and
writes, a single function (e.g. `a`) is still guaranteed to execute sequentially
with.

With all that in mind, our definition for the end state of running our functions
is the values of all memory locations written by at least one of the functions
and read by at least one different function. We can also call the set of
addresses containing end states the *shared memory* between our functions.

## Collecting End States

One approach to acquiring these addresses is via user input. When specifying the
functions to check, the user of our tool could also specify a set of memory
addresses for our shared memory. However, there are a number of downsides to
this approach
1. User error. Under- or over-specifying the shared memory set could produce
   incorrect results. Under-specification means the checker might miss some
   incorrect interleavings. Over-specification means the checker might
   unnecessarily flag correct code as incorrect because functions *not* under
   study modify shared memory.
2. Performance. Over-specification of shared memory means more memory is
   checked. For small amounts this is negligible, but in larger quantities
   excess checking can add up across all interleavings.
3. Convenience. Larger programs could have many, many (memory allocation,
   concurrent datastructures, synchronization primitives, etc.) pieces of shared
   memory and they may be scattered across the address space. Additionally,
   their addresses may have to be computed at runtime (heap allocated
   datastructures, for example). Manual specification is difficult and possibly
   error prone.

Instead, the Thread Equivalence Checker supports automatic detection of shared
memory. The detected shared memory (if enabled) is unioned with user supplied
shared memory (if specified). Automatic shared memory detection is done with the
Armv7's MMU security features.

A full description of the Armv7 virtual memory system or virtual memory in
general is much too long for this section, weblog, or series. Maybe I'll
write a weblog on it in the future.

Instead, I'll cover how we used the memory system to perform automatic shared
memory detection.

Based on our definition above, we can compute the shared memory set with set
operations alone so long as we have the set of memory addresses written by each
function and read by each function. To find that we configure our address space,
MMU, and fault handlers such that any reads or writes when we are executing a
function under test trigger a data abort. From within that data abort we observe
the instruction that made the request (since data aborts report only a single
byte that caused the abort we need to decode the instruction to determine the
remaining bytes) and the address it attempted to access.

**tl;dr the rough steps for doing this are**:
1. Compile functions under study into seperate section and move them somewhere
   far (>1MB) away from checker code at startup
2. Set up pinned TLB entries that map all code to itself (KISS). Give the
   checker sections a different *domain* than the function under study
   section(s). Only function **code** is given the second domain. Give the
   function under study section(s) read/write priveleged permissions (`AP` &
   `APX`) and make them global.
3. When you want to catch a read/write: set the domain access of the second
   domain to `client` (instead of `manager`, which disables permission checks),
   then context switch into `USER` mode (out of the default `SUPER`) while
   simultaneously jumping to testing code. Now accesses to sections in the
   kernel domain will trigger section permission faults, which raise data aborts
   (because, importantly, instruction fetches for function code will not trigger
   permission faults thanks to our seperate sections). The data abort handler
   will also avoid triggering faults because the permissions will not be checked
   in the abort handler mode.

Once in the data abort, figuring out the complete set of addresses accessed is a
task in Arm instruction decoding. Here is the function we use, complete with
annotations from the manual (e.g. `A4-213`):

```c
static void get_touched_bytes(uint32_t instruction, uint32_t addr, set_t* destination_set) {
  // A4-213 SWP
  if(bits_get(instruction, 20, 27) == 0b00010000) {
    for(int i = 0; i < 4; i++)
      set_insert(destination_set, addr + i);
  }
  // A4-214 SWPB
  else if(bits_get(instruction, 20, 27) == 0b00010100) {
    set_insert(destination_set, addr);
  }
  // A4-52 & A4-202 LDREX/STREXX
  else if(bits_get(instruction, 21, 27) == 0b0001100) {
    // Only words
    for(int i = 0; i < 4; i++)
      set_insert(destination_set, addr + i);
  }
  // A3-22 : Load/store word or unsigned byte
  else if(bits_get(instruction, 26, 27) == 0b01) {
    // A3-22 : B == 1 means byte
    if(bit_isset(instruction, 22)) {
      set_insert(destination_set, addr);
    } else {
      for(int i = 0; i < 4; i++)
        set_insert(destination_set, addr + i);
    }
  }
  // A3-23 : Load/store halfword, double word, or signed byte
  else if(bits_get(instruction, 25, 27) == 0b000) {
    uint32_t l = bit_isset(instruction, 20);
    uint32_t sh = bits_get(instruction, 5, 6);
    uint32_t lsh = (l << 2) | sh;
    // A5-34
    switch(lsh) {
      // Store halfword
      case 0b001:
      // Load signed half word
      case 0b111:
      // Load unsigned halfword
      case 0b101:
        set_insert(destination_set, addr);
        set_insert(destination_set, addr + 1);
        break;
      // Load double word
      case 0b010:
      // Store double word
      case 0b011:
        for(int i = 0; i < 8; i++)
          set_insert(destination_set, addr + i);
        break;
      // Load signed byte
      case 0b110:
        set_insert(destination_set, addr);
        break;
      default:
        printk("%x accessed %x\n", instruction, addr);
        panic("Unexpected LSH combination\n");
    }
  }
  // A3-26 : Load/store multiple
  else if(bits_get(instruction, 25, 27) == 0b100) {
    // Weakness - assumes that the LDM/STM traps for ALL accessed data, not
    // only some of the accesses. Generally true unless on section boundary
    uint32_t register_list = bits_get(instruction, 0, 15);
    uint32_t offset = 0;
    for(int i = 0; i < 16; i++) {
      if((register_list >> i) & 0x1) {
        for(int j = 0; j < 4; j++)
          set_insert(destination_set, addr + offset + j);
        offset += 4;
      }
    }
  }
}
```

You also have to extract the relevant data from coprocessor registers (cp 15),
and saved registers on the stack. Here is how we did that:

```c
uint32_t addr = cp15_far_get();
uint32_t pc = r->regs[REGS_PC];
uint32_t dfsr = cp15_dfsr_get();

// Parse DFSR according to B4-43
uint32_t status = (bit_isset(dfsr, 10) << 4) | bits_get(dfsr, 0, 3);
demand(status == 0b01101, only section permission faults expected);
uint32_t domain = bits_get(dfsr, 4, 7);
demand(domain != user_dom, we should never fault when accessing the user domain);

// Read/write bit is stored here in addition to the instruction but this is
// much more consistent
uint32_t w = bit_isset(dfsr, 11);
```

## Aside: Arm MMU Security: Domains, Permissions, and Modes

The Armv7 architecture has a few interlocking ways of protecting memory and
execution state. I debated whether to include a section on them here. For
brevity, I decided against it. Along with the Armv7 virtual memory system (and
virtual memory in general), it is best left as a seperate weblog.

## Aside: Address Sets

So far, we've seen sets of addresses unioned and intersected. We've also seen
addresses inserted into sets without care for duplication. Since we could not
find a set implementation that suited our needs (efficientely represents sparse
sets across the entire 32 bit address space) The Thread Equivalence Checker
makes use of a custom set implementation.

Our set data structure needs to support the following:
1. Quick insert and lookup
2. Relatively quick union and intersection
3. Relatively quick enumeration of all elements
4. Space efficient for sparse clumps of dense addresses (e.g. heap access
   patterns)
5. Able to represent the entire 32-bit address space

We chose a tree. Each node contains a 32-bit mask and 32 children
pointers. Each level of the tree further partitions the address space into 32
pieces. Thus, we need `ceil(log(2**32, 32)) = 7` levels for the tree. Each
grouping of `log(32, 2) = 5` bits in an address is used as an index into a nodes
children (or index into the mask if we are at a leaf node). For example:

```
v = 01 00010 00100 00101 00101 00110 00111
```

(Note that there is some overhand and we want to put that in the root node to
save space)

To insert `v` into the set, we lookup the second child of the root,third child
of that child, fifth child of that child, and so on.  The mask of each node
indicates whether that child is "present," so if it is 0 we allocate it before
moving on. Finally, at the leaf node, we lookup the eighth bit of the mask and
set it if it is not already set.

To lookup `v`, we perform the same recursive descent. However, we can return
`false` early if a mask bit is ever 0.

Intersection and union are similarly done by or-ing and and-ing masks.

Improvements could definetly be made to the set implementation, though its
simplicity was the main attraction at the time. For example, one notable
improvement is better memory layout of sets to reduce cache misses when
processing large sets and improved subtree reuse 

## Aside: Heap

Our set implementation demands heap allocation. We decided to write a small
explicit coalescing heap allocator. An alternative would have been arena
allocation. The implementation of the heap allocator was not nearly as
interesting as the rest of the equivalence checker, so it is mentioned here for
completeness.

## Comparing End States

Now that we have a shared memory set, how do we compare them? Once we execute a
sequence of functions or an interleaving, the shared memory will contain some
values. We can read each value in a deterministic order (in our case, in
order of increasing address) into a hash function then compare hashes. Since our
hashes are 32 bits, we can reuse our generic set to maintain a set of sequential
hashes (generated in the first step: collect sequential end states) and check
after each interleaving whether the hash of shared memory is in the set. With a
good enough hash function, this works well.

The hash function we used is 32-bit xxHash.

## Stay Tuned

In part 3, I'll finally cover the core of the checker: executing interleavings
using Armv7 debugging hardware.

