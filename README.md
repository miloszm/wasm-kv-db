# wasm-kv-db

A lightweight, embeddable key-value database with Wasm guest support for custom logic

## Introduction

Several existing database systems allow users to submit and run Wasm code. Such code is sometimes referred to as
a User-Defined Function (UDF).
To such database systems belong SingleStore, libSQL, ClickHouse, SpacetimeDB.
There are also browser-first databases like Isar Plus, Lattice DB, MoltenDb.

User-submitted Wasm is a core part of SpacetimeDB architecture. Instead of writing
SQL or some query language, you write your backend logic as a Wasm module.
Wasm modules define schema and all logic, and run directly inside the database.
Such approach has the advantage of zero-latency local queries, especially if the
database is cached, as well as many other advantages.
Wasm modules are executed securely, with constraints like "fuel" to limit the 
CPU usage.

Wasm modules running inside databases have been assigned a bit misleading name - "reducers".
In essence, reducers are modules realizing function:

`f(s, a) => s'`

where **s** is a state, **a** is some set of actions, and **s'** is the next state.
Thus, "reducers" are not actually reducing, as would the name suggest, i.e., are not performing
a function **f(v[..]) => scalar**. Nevertheless, the name "reducer" has caught on.
We will use the name **reducer** in this project as well.

What follows is a vision behind wasm-kv-db. It is presented in a rather long list of points,
but these points should make the intention behind the system clear and provide essential information
about what need and functionality the project addresses:
- reducers are Wasm functions, that form Wasm guest
- Wasm guest is a Wasm module submitted by tenant, containing a set of reducers
- at any point in time, there is one Wasm module (guest) per tenant
- guests must contain reducers and expose a special function execute that dispatches calls to reducers
- tenants can see the database as their own area, independent of other tenants
- only reducers can read or write from/to the database
- reducers run sequentially, one at a time, in an uninterrupted way
- database is key-value and all access to it is cached via DashMap
- DashMap contains all the content at all times
- all content of the DashMap cache is continually written to the RocksDB database in the background thread
- cache is reloaded from persistent store (RocksDB) when the system is restarted
- the system host provides data manipulation (host) functions to be used by the Wasm guest and its reducers
- the system contains HTTP server through which users can submit reducer calls and obtain responses
- there is a reference CLI tool to drive an example raffle application
- there is reference raffle guest code provided that allows to create a raffle, buy tickets and draw a winner
- raffle functionality is demonstrated in guest unit test as well as in the CLI tool
- all data is passed via msgpack serialization format
- guest always knows on behalf of which user it is running and can act accordingly (via the host function __caller__)

The system is not aware of what applications its tenants are running. It allows users to pass byte buffers
of serialized arguments and return values to/from executors provided by the guests. Only tenants/users
and the guest code is aware of what user functionality the hosting database system is realizing. This approach allows,
on the one hand, to amalgamate the database with the business logic (database 'is' the application),
and on the other hand, to make the database proper oblivious to business logic, data types, queries, etc.

There is no middle tier. Instead of a client talking to a server that talks to a DB,
here the clients call their applications' reducers directly at the database. 
Latency and complexity are drastically reduced.

Reducers run sequentially, one at a time, there are no race conditions, no risk of
two user overwriting each other's data.

Because of the cache, reads and writes are extremely fast. All data is kept in the cache
and is stored in RocksDB in a background thread for consistency.

It is easy to replay the operations as reducers are pure functions `f(s,a) => s'`

Reducers give you ironclad guarantees, as only they are allowed to manipulate data. All writes
go through a Wasm guest with reducers, there is no other way of writing data.



## Features

What followis is a list of major features of wasm-kv-db:

- Fast in-memory cache (DashMap)
- Wasm guest support with host functions accessing data
- RocksDB Persistence
- REST API
- Multi-tenancy
- Reference CLI and reference Wasm guest for the Raffle application

## ToDos

Simple but important TODOs:
1) Mechanism to upload guests is still not done. Currently, function execute
in handlers.rs has the path to the guest hardcoded.

2) Tenancy in host functions needs to be taken care of.

Nice to have:
Example cli-client lacks queries or deletion of raffles.
Example guest also could allow cleaning up of raffles.

## Possible Future Enhancements

- Add a concept of "fuel" to constrain CPU usage

## Quick Start

After git clone you can run cargo r in the main folder to start the HTTP server.

After that you can cd to the CLI folder, build the CLI and start using it.
Here is an example of a CLI session:

```
$> cd wasm-guests/raffle

$> cargo build --target wasm32-unknown-unknown

$> cd ../..

$> cd cli-client

$> cargo b

$> cd target/debug

$> ./raffle-cli --caller admin create --raffle-id raffle_1 --total-tickets 100
Raffle raffle_1 created with 100 tickets

$> ./raffle-cli --caller user_001 buy --raffle-id raffle_1 --quantity 1
Purchased 1 ticket(s) 99 remaining

$> ./raffle-cli --caller user_002 buy --raffle-id raffle_1 --quantity 2
Purchased 2 ticket(s) 97 remaining

$> ./raffle-cli --caller admin draw --raffle-id raffle_1
Winner: user_002

$> cd ../../..

$> cargo t

```

note guest upload is not provided yet so for now you need to edit the path in handlers.rs to deploy
your own guest (this will be done ASAP - time permitting)

## Architecture

- Central point of the database is the DashMap containing all data all the time.
- HTTP server allows for passing reducer names and their arguments to particular tenants' guest code.
- HTTP handlers allow for a limited read/only access to data, but primarily provide a mechanism to call the **execute** method of the guest.
- Storage module provides the DashMap cache as well as the persistence layer based on RocksDB.
- On every cache modification, the modification operation is being forwarded to the background thread and batched there for eventual flush into the persistent database.
- Wasm module instantiates guests, imports the execute function, and provides several host functions to the guest.

Here is the main flow that illustrates the workings of the system:

HTTP --> WASM --> WASM_GUEST --> HOST --> DASH_MAP_CACHE --> PERSISTENT_DB


## Host Functions

The following hosts functions are available for the guest Wasm code:

```rust
    fn host_put(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32
    fn host_put_int(key_ptr: *const u8, key_len: usize, value: i64) -> i32
    fn host_get(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32
    fn host_get_len(key_ptr: *const u8, key_len: usize) -> i32
    fn host_get_int(key_ptr: *const u8, key_len: usize) -> i64
    fn host_append_to_list(
        key_ptr: *const u8,
        key_len: usize,
        value_ptr: *const u8,
        value_len: usize,
    ) -> i32
    fn host_caller(caller_ptr: *const u8, caller_len: usize) -> i32
    fn host_rand(max: u32) -> u32
```

## Examples

There are two examples of a guest provided, a simple guest which demonstrates basic reading and writing
of data, and a more complex guest implementing a raffle application. The raffle application allows
a raffle to be created, tickets bought, and then to draw a random winner with chances proportional
to the number of tickets purchased. This is a simple yet self-contained and complete application
demonstrating the possibility of implementing business login inside the database, without the database
being aware of it.

## REST API Reference

The following API is provided:

GET  /health            - health check of the HTTP service
GET  /kv                - returns a list of all keys in the cache, independent of tenant
GET  /kv/{tenant}/{key} - returns value of particular key
POST /kvexec            - executes particular reducer, see below for more information

kvexec requires the following structure to be sent in a MessagePack-serialized format.

```rust
pub struct GenericRequest {
    pub tenant_id: String,
    pub reducer_name: String,
    pub reducer_args: Vec<u8>, // MessagePack encoded arguments to the reducer
    pub caller_id: String,     // will be provided via host_caller to the guest code
    pub timestamp: u64,        // currently not used, may be 0, eventually passed by the host function
}
```

The following simple example session illustrates the use of the api: 
```

$> ./raffle-cli --caller admin create --raffle-id raffle_1 --total-tickets 100
Raffle raffle_1 created with 100 tickets

$> curl localhost:8080/kv
["raffle:raffle_1:end_time","raffle:raffle_1:tickets_left","raffle:raffle_1:entries"]

$> curl localhost:8080/kv//raffle:raffle_1:tickets_left
[100,0,0,0,0,0,0,0]
```

Note that in the last example tenant is empty.
Note that to use kvexec, because of its serialized body structure requirement, you need to use the provided cli_client, for example as follows:

```
$> cd cli_client

$> cargo b

$> cd target/debug

$> ./raffle-cli --caller admin create --raffle-id raffle_1 --total-tickets 100
Raffle raffle_1 created with 100 tickets

```
