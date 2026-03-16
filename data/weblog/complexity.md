Complex systems appear everywhere, and for good reason. In biology, everything from cells to ants is part of a complex system. Arguably, the biosphere itself is one large complex system. In sociology, where humans are the individual unit, people self-assemble into tremendous entities like towns, corporations, and countries. In digital systems, complexity reaches a similarly unfathomable scale: modern information systems are tall hierarchies of abstractions, each a composition of disparate and asynchronous components. The internet is built of billions of computers, each a composition of advanced processors. Each processor is a composition of hundreds or thousands of individual silicon blocks. In all cases, complexity buys _expressive power_.

As engineers, we want to build and change these complex digital systems. Unfortunately, our efforts to work with them are frequently stymied by the sheer complexity involved. The central issue is not just that these systems are large or intricate, but more that the surface area required to reason about them grows faster than the abstractions, tools, and organizational structures we use to manage them. In particular, our tools for specifying and modeling complex computer systems are underpowered. Here I present three, not necessarily orthogonal, aspects of this problem to consider. The ideal, though not necessarily realizable, tool would model all three aspects of a system and permit an engineer to reason about them.

## Aspect 1: Temporal Complexity

### Definition

Imagine you have a purely sequential system. It can be represented as a state machine with `N` states, and in each time step your state machine transitions to the next state. You can increase the power of your system by running `M` state machines in parallel. To model your new increase in power, one might use a larger state machine of `N^M` states. Unfortunately, this abstraction makes an incorrect assumption: that all state machines transition in _lockstep_. Practically speaking, this may be true _most of the time_ if you consider a granular enough transition, for example, "every clock cycle" for a processor. However, for sufficiently complex systems, the _granularity_ of state transitions becomes prohibitively small.

What we are facing is a combinatorial explosion in states and their transitions as a result of parallelization in time. The mere act of running two or more sequential processes in parallel introduces questions about the ordering of events across the whole system and the visibility of events from the perspective of each sequential process. Attempts to model the true state machine quickly become intractable for any useful system.

### In Silicon

The PDP-11 had one huge memory-mapped I/O bus and a CPU, a platonic ideal for a Von Neumann machine. Unix and C were built around this. In contrast, a single x86 core might have _hundreds_ of instructions _in flight_, all of which could complete _on different cycles_. With dynamic voltage and frequency scaling, even the notion of a cycle is suspect. Enormous complexity arises when trying to reason about the latter with the _same tools_ used for the former: existing verification tools like formal methods, simulation, and FPGA emulation are limited to small silicon blocks. Moreover, a GPU or CPU might now have hundreds of _cores_, each with _hundreds of instructions_ in flight. What is the total state space now?

Failures in reasoning through the whole system lead to bugs like Spectre and Meltdown. In Spectre's case, a specific combination of conditions created a side-channel attack by exposing side effects of branch speculation. This particular bug resulted from a system state that emerges from a particular execution ordering and timing. This class of vulnerability remains an ongoing problem and continues to impact almost every computer system.

Vulnerabilities like Spectre serve to underscore both the lack of tooling to accurately specify, model, and verify complex hardware systems and the importance of such tooling.

### In Software

Sitting one level above the hardware is software, and it has not escaped this issue. To further parallelize, we network many machines, each with many processors, many cores, and many instructions in flight, together. Reasoning about a software system, even if the underlying hardware maintains the illusion of perfectly sequential and atomic execution, still requires timing information. What if the CPU of machine A starts 2 ms ahead of B and the packets from B arrive slightly after A and thus get put into a different address in the buffer?

From a programming language perspective, our languages predominantly stem from either the Turing or Church tradition of design. The Turing tradition makes a strong assumption about the underlying execution model: it is a sequential state machine operating on linear memory. Although we have successfully applied the Church tradition's notion of typing to enforce behavioral contracts between subroutines, type systems are limited in their power to specify temporal constraints.

### Tooling

As a concrete example common to both hardware and software, consider memory models. A memory model attempts (with varying success) to completely specify the interaction of memory operations issued from multiple sequential actors (e.g. threads, processors, etc.). Given a thorough memory model implemented faithfully by the underlying abstraction (e.g. the memory architecture, OS, etc.), concurrent memory actors can be fully reasoned about.

A higher-level behavioral specification _with a corresponding verifier_ similar to the memory models of low-level systems is sorely needed. One notable mention is TLA+, but a significant shortcoming is the divergence between the TLA+ model of an algorithm and its implementation (such a divergence might result from aspect 2).

## Aspect 2: Organizational Complexity

### Definition

Suppose you are designing one of the modern CPUs described in aspect 1. No single human being could possibly understand every detail of every piece, so you form many teams of engineers to design individual components. Unfortunately, the introduction of multiple humans into the process creates _organizational_ challenges. For example, it becomes difficult to get everyone to use the same tools and maintain a consistent mental or documented model. For a sufficiently complex project, it could be the case that no single person understands the entire system. As the behavioral surface area of the system grows, the communication bandwidth and shared mental models of the organization do not scale proportionally, and global reasoning becomes fragmented across teams.

### Challenges of Human Organization

Organization of humans is a fundamental problem of human society. It could be argued that it is _the_ defining problem of human society. I do not claim to be able to fully characterize the problems present in human organization, but it is clear that cooperative problem solving induces additional complexity in organizing the people involved. In fact, it has been argued that the complexity flows the other way. Conway's Law is a common rule of thumb that argues systems reflect the organizational hierarchies that created them.

Organizational engineering remains a huge open problem on which all human organizations have their own opinions. In general, flat hierarchies struggle to form consensus, that is, to move information horizontally, and tall hierarchies struggle to inform decision makers at the top with information from below, that is, to move information vertically. For example, Amazon tries to solve the communication problem with a controversial amount of documentation and meetings, and Apple tries to decentralize power and flatten the hierarchy by endowing directly responsible individuals with decision-making power on projects. In both cases, the organization is compensating for limits in shared reasoning capacity, either by increasing documentation overhead or by localizing authority. Neither approach restores a global, unified model of the system.

### Challenges of AI Organization

A recent development in this problem space has been the removal of the human from human organization with AI, in particular generative LLMs. Unfortunately, LLMs seem to suffer from some of the same problems humans do. A recent work out of Stanford measured how effective LLMs were at communicating over a text channel to cooperate on a code editing task. The results showed that the models quickly failed to recognize the relationship between the messages and the actions of other models. Strict hierarchies remain the gold standard for LLM agent orchestration. Perhaps this says something about the underlying human data on which the models were trained.

### Tooling

The consequences of poor organizational design are most apparent at the boundaries between components. In line with Conway's Law, we would expect each component to reflect the skills of the team that made it. This might mean that different teams implement the TLA+ spec of a component and the component's implementation, creating an opportunity for divergence. If the spec no longer matches the implementation, the spec has tremendously less value. What is needed is an expressive specification language and corresponding verification method for the boundaries between components. Such a tool must match or exceed the communication bandwidth between the teams developing each component to avoid becoming a bottleneck.

## Aspect 3: Compositional Complexity

### Definition

Emergent properties are global properties that are not easily derivable without explicit simulation. For example, the complex computational patterns that emerge from the simple rules of Conway's Game of Life are commonly called _emergent_. Of course, those complex computational patterns are completely predictable via simulation, but a priori they are not clearly the result of applying the simple rules. Emergent properties are themselves an example of _compositional complexity_, complexity that arises from the composition of components into a system. Our specification and verification tools typically operate at the level of individual components, but the properties we care about often exist only at the level of the composed system.

### In Computation

Functional programming languages seek to define pure programs that are well specified. In isolation, these programs compose cleanly. In practice, however, the systems that compose and execute them are impure. Though the internal behavior of the executable is well defined by the language, the surrounding environment that orchestrates their interaction, such as the operating system or network, reintroduces the temporal and organizational complexities discussed above.

Hardware is similar. Each component might be well specified and even formally verified, but their composition could still produce Spectre. Moreover, in both software and hardware, there is an overwhelming trend toward _specialization_. With each component highly specialized and with the introduction of a wide variety of component types, reasoning about the system goes beyond the simple multiplicative effects of aspect 1. Entirely novel behaviors now emerge. As an example, consider a dataflow architecture: each stage of a programmable dataflow pipeline may be simple and well verified, but the composition is user defined and capable of much more complex behavior. The reasoning guarantees proved locally do not automatically lift to the global system, and the gap between local correctness and global behavior widens as specialization increases.

### Tooling

What is needed is a tool that can, given some form of expressive specification for each component, derive bounds on the emergent properties of their composition. Unfortunately, the nature of emergent properties (and some constraints inherent to general computation like Rice's Theorem) severely limit what such a tool would be capable of. Instead, it would be worth exploring how the behavior of each component could be _constrained_ to better reason about the whole.

## Perfect Storms

Temporal parallelism, organizational fragmentation, and compositional emergence each expand the system’s behavioral surface area in ways that outpace the abstractions we use to reason about it. Now, we will see two examples of how these aspects manifest and interact.

### Microservices

Microservices perfectly reflect all three aspects. Each service communicates, potentially asynchronously, across lossy networks, such as pub/sub or RPC. Additionally, the entire system is designed to be written using different tools and across different teams. Finally, although each microservice may be well tested, there are limits to the testing that can happen for the entire system at production scale.

Moreover, the specification for a microservice architecture rarely materializes in any formal sense. Instead, it is likely, and often intentional, that each service be implemented with different tools and thus have mutually incompatible behavioral contracts. Instead, the services meet at the lowest common denominator, something like the gRPC DSL or Kubernetes or Helm YAML files, which are essentially DSLs for specifying microservices using RPC and streaming primitives. None of these DSLs support fine grained behavioral analysis. Even if a TLA+ specification is tractable, the actual implementations could diverge from the proven TLA+ specification.

### Specialized Accelerators

Specialized hardware combines massive temporal complexity, many thousands of individual computation units, and compositional complexity, many specialized units, for example, deep cache hierarchies creating performance cliffs. Moreover, specialized accelerators beget a form of organizational complexity: software hardware codesign. The contract, that is, decades old ISAs, between hardware designers and software programmers has been ripped up in service of ever greater performance. The result is that the tools for working with these devices, usually debuggers and compilers, are leaky and require reasoning about the entire system.

## Conclusion

In surveying temporal, organizational, and compositional complexity across hardware, software, and the institutions that produce them, a common theme emerges: our capacity to construct complex systems has far outpaced our capacity to reason about them. Parallelism in time explodes state spaces beyond tractable modeling. Coordination among humans and machines introduces structural distortions that shape the systems themselves. Composition yields emergent behaviors that evade local specification and verification. When these forces converge in modern architectures such as microservices and specialized accelerators, the result is a perfect storm in which informal contracts, leaky abstractions, and partial tools stand in for comprehensive understanding. If complexity indeed buys expressive power, then the central engineering challenge is not merely to build ever more expressive systems, but to develop methods, languages, and organizational forms capable of containing the complexity we have unleashed.
