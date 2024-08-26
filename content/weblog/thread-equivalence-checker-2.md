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
writes, a single function (e.g. `a`) is still guaranteed to execute sequentially.

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
Armv6's MMU security features.

A full description of the Armv6 virtual memory system or virtual memory in
general is much too long for this section, weblog, or series. Maybe I'll
write a weblog on it in the future.

Instead, I'll cover how we used the memory system to perform automatic shared
memory detection.

### Arm Virtual Memory System Security: Domains, Permissions, and Modes

The Armv6 architecture has a few interlocking ways of protecting memory and
execution state.

#### Modes

The ARM core is always executing in one of 7 processor modes:
- User (`usr`)
- System (`sys`)
- FIQ (`fiq`)
- IRQ (`irq`)
- Supervisor (`svc`)
- Abort (`abt`)
- Undefined (`und`)

The first, user mode, is the only unprivileged mode. The processor cannot leave
this mode without raising an exception, and certain operations are prohibited.

The remaining modes serve various purposes but all are privileged, so they can
switch mode by writing to system registers and perform any supported operation.

The important take away for our application is that the core can be either
running in a privileged or unprivileged (user) mode and that exceptions always execute
in a privileged mode (`abt`).

#### Permissions

Each TLB entry contains 5 permission bits (esoterically named `S`, `R`, `APX`,
and the two bit `AP`) that together form the access permissions for the entry.
Entries have seperate controls for privileged and user accesses, though
user access implies privileged access. For example, an entry could be
read/write access for privileged modes but read only access for user mode.

#### Domains

The final piece of the security scheme we use is domains. The domain system
serves as a quick way to enable and disable the permission scheme described
above without having to change permission bits on page table entries (which
could mean invalidating large swathes of the TLB).

ARM MMU (TLB & page table) entries each are assigned one of 16 domains. A single
register, the Domain Access Control Register, controls access to each of the 16
domains via a two bit access control field. A domain can either be no access,
client, or manager (the fourth possible value is reserved and undefined). Any
accesses to memory mapped by an entry in a no access domain fault. If the domain
is a manager, however, the accesses are *always allowed*. Finally, if the domain
is a client, the above permission bits are checked.

### Catching Reads and Writes

Based on our definition for shared memory, we can compute the shared memory set
with set operations alone so long as we have the set of memory addresses written
by each function and read by each function. To collect those addresses,
collectively called the read-write sets, we can use the above permissioning
scheme to trigger data aborts exclusively when we are executing code we are
checking.

First, we need to compile checked code into seperate section and move them
somewhere far (>1MB) away from checker code at startup. This sets up the address
space so that we can use coarse grained sections (instead of finer grain pages)
and TLB pinning (which is easier to deal with than full page tables and works
just as well for us).

Next, we set up pinned TLB entries that map all code to itself. We give the
checker sections a different *domain* than the checked section(s). Only checked
**code** is given the second domain (this ensures instruction fetches do not
fault). Give the checked section(s) read/write privileged permissions and make
them global. Recall that these permissions are only ever checked if the domain
of the section is a client, not a manager. Initialize the domains to all be
managers to begin with.

Now, when we want to catch a read/write: set the domain access of the second
domain to client, then context switch into user mode while simultaneously
jumping to testing code. This is best accomplished by running functions as their
own threads and context switching. Now accesses to sections in the checker
domain (which includes all global data) will trigger section permission faults,
which raise data aborts. The data abort handler will not fault because the
permissions will not be checked when in the privileged abort handler mode.

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

## Aside: Address Sets

Since we could not find a set implementation that suited our needs (efficiently
represents sparse sets across the entire 32 bit address space) the Thread
Equivalence Checker uses a custom set implementation.

Our set data structure needs to support the following:
1. Quick insert and lookup
2. Relatively quick union and intersection
3. Relatively quick enumeration of all elements
4. Space efficient for sparse clumps of dense addresses (e.g. heap access
   patterns)
5. Able to represent the entire 32-bit address space

We chose a tree. Each node contains a 32-bit mask and 32 children
pointers. Each level of the tree partitions the address space into 32
pieces. Thus, we need `ceil(log(2**32, 32)) = 7` levels for the tree.

Each grouping of `log(32, 2) = 5` bits in an address is used as an index into a
nodes children (or index into the mask if we are at a leaf node). Each node has
an `offset` field that stores the bit offset of this grouping. For example:

```
v       = 01 00010 00100 00101 00101 00110 00111
        =  1   2     4     5     5     6     7
offset  = 30   25    20    15    10    5     0
```

(Note that there is some overhand and we want to put that in the root node to
save space)

To insert `v` into the set, we lookup the second child of the root, third child
of that child, fifth child of that child, and so on.  The mask of each node
indicates whether that child is "present," so if it is 0 we allocate it before
moving on. Finally, at the leaf node, we lookup the eighth bit of the mask and
set it if it is not already set.

```c
uint32_t set_insert(set_t* s, uint32_t v) {
  uint32_t index = (v >> s->offset) & 0x1F;

  uint32_t bit = 0x1 << index;
  uint32_t present = s->mask & bit;

  s->mask |= bit;

  // If this isn't a leaf node, we need to recurse
  if(s->offset > 0) {
    // If the child doesn't exist, make it
    if(!present) {
      s->children[index] = set_alloc_offset(s->offset - 5);
    }
    return set_insert(s->children[index], v);
  }

  return present;
}
```

To lookup `v`, we perform the same recursive descent. However, we can return
`false` early if a mask bit is ever 0.

```c
uint32_t set_lookup(set_t* s, uint32_t v) {
  uint32_t index = (v >> s->offset) & 0x1F;

  uint32_t bit = 0x1 << index;
  uint32_t present = s->mask & bit;

  // If the prefix is not present in the set, give up
  if(!present) return 0;

  // If the prefix is present and we are a leaf node, return the present bit
  if(s->offset == 0) return present >> index;

  // Otherwise recurse
  return set_lookup(s->children[index], v);
}
```

To perform a union of two sets `x` and `y`, we merge subtrees by ORing the masks
and recursing if `x` and `y` both have a child `i`.

```c
void set_union(set_t* z, set_t* x, set_t* y) {
  assert(z->offset == x->offset && x->offset == y->offset);

  z->mask = x->mask | y->mask;

  if(z->offset == 0) return;

  uint32_t both_present = x->mask & y->mask;
  uint32_t at_least_one_present = x->mask | y->mask;
  for(int i = 0; i < 32; i++) {
    if(mask_has(both_present, i)) {
      // Make the child
      z->children[i] = set_alloc_offset(z->offset - 5);

      // Recursively call union
      set_union(z->children[i], x->children[i], y->children[i]);
    } else if(mask_has(at_least_one_present, i)) {
      // Make the child
      z->children[i] = set_alloc_offset(z->offset - 5);

      // Just take the one that is present
      if(mask_has(x->mask, i)) {
        set_copy(z->children[i], x->children[i]);
      } else if(mask_has(y->mask, i)) {
        set_copy(z->children[i], y->children[i]);
      }
    }
  }
}
```

For intersections we do something similar, merging subtrees by ANDing the masks
and recursing if both `x` and `y` have a child `i`.

```c
void set_intersection(set_t* z, set_t* x, set_t* y) {
  assert(z->offset == x->offset && x->offset == y->offset);
  
  uint32_t both_present = y->mask & x->mask;
  z->mask = both_present;

  if(z->offset == 0) return;

  for(int i = 0; i < 32; i++) {
    if(mask_has(both_present, i)) {
      z->children[i] = set_alloc_offset(x->offset - 5);
      set_intersection(z->children[i], x->children[i], y->children[i]);
    }
  }
}
```

Finally, an inorder traversal of the tree orders set elements by increasing
value.

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
using Armv6 debugging hardware.

[Part 3]({{< relref "thread-equivalence-checker-3" >}})

