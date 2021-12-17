# Introduction

Sia is a simple distributed systems communication framework.
It provides a simple interface for you to code without having
to worry about the communication backend and more.

Sia's core principles are a high-level of abstraction and minimalism,
while keeping configurability and flexibility. Sia also strives
to be a zero-cost abstraction, offering as much performance as possible.

Sia is not opinionated on project structure and only provides tools and more
to make it easier to create distributed systems.

Unlike most distributed systems libraries, Sia has object stream-based communication
instead of rpc-based communication. This means that Sia is usually more performant than
other similar solutions such as gRPC and is comparable to websockets, but is more ergonomic
and usually performs better than websockets.

Still, this library may be a little too low-level for *some* use cases.

Should your project use Sia?
Sia fits these use cases perfectly.

- You want to build an RPC system
- You want to build a consensus algorithm
- You want to build a distributed system
- You want to build a library for distributed systems
- Anything that's got to do with distributed systems or communications

**DISCLAIMER**

Sia is not yet on 1.0, which means that the API is prone to changes. Nevertheless,
the concepts should be stable **BUT** they are still prone to changes.
