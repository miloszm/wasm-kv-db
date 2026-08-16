# wasm-kv-db

A lightweight, embeddable key-value database with Wasm guest support for custom logic

## Introduction

Several databses allow users to submit and run Wasm code, sometimes referred to as
User-Defined Functions (UDFs).
To such databases belong SingleStore, libSQL, ClickHouse, SpacetimeDB.
There are also browser-first databases like Isar Plus, Lattice DB, MoltenDb.

User-submitted Wasm is a core part of SpacetimeDB architecture. Instead of writing
SQL or some query language, you write your backend logic as a Wasm module.
Wasm modules define schema and all logic, and run directly inside the database.
Such approach has the advantage of zero-latency local queries, especially if the
database is cached.
Wasm modules are executed securely, with constraints like "fuel" to limit the 
CPU usage.

Wasm modules running inside the database have a bit misleading name "reducers".
In essence, reducers are modules realizing function:

`f(s, a) => s'`

where s is a state, a is some set of actions, and s' is the next, changed state.
Thus, "reducers" are not reducing anything, as the name rather suggests
a function f(v[..]) => scalar. Nevertheless, the name "reducer" caught on and
it will be used in this project as well.

This project realizes the following vision:
- reducers are Wasm functions, which together for a Wasm guest or Wasm module
- there is one Wasm module per tenant
- tenancies are conceptually separate database realms, so tenants can see the database as their own area, independent from other tenants
- only reducers can read or write database
- database is key-value and all access is cached via DashMap
- database is persistent, all content of cache is continually written to the RocksDB database
- cache is reloaded when system is restarted
- deployed modules must expose the "execute" function
- system host provides data manipulation functions to the Wasm guest and its reducers
- the system also runs HTTP web server through which users can submit reducer calls
- reference CLI tool to drive an example raffle application is provided
- reference raffle guest code is provided and allows to create raffle, buy tickets and draw a winner
- raffle functionality is shown in guest-test and via the CLI tool
- all data is passed via msgpack serialization format
- guest always knows on behalf of which user it is running

The system is not aware of which applications its tenants are running. It allows users to pass byte buffers
of serialized arguments and return values to executors provided by guests. Only tenant users
and the guest code knows what user functionality the hosting database system is realizing.

This approach allows database to amalgamate with business logic.
Code and data are one, the database 'is' the application, or many applications.

There is no middle tier. Instead of a client talking to a server that talks to a DB,
here the client calls a reducer directly on the DB. Latency and complexity are
drastically reduced.

Reducers run sequentially, one at a time, there are no race condiction, no risk of
two user overwriting each other's data.

Because of the cache, reads and writes are extremely fast. All data is kept in the cache
and is stored in RocksDB in a background thread for consistency.

It is easy to replay the operations as reducers are pure functions `f(s,a) => s'`

Reducers give you ironclad guarantees, as only they manipulate data. All writes
go through a Wasm guest with reducers, there is no other way of writing data.





## Features

- Fast in-memory cache (DashMap)
- Wasm guest support with host functions accessing data
- RocksDB Persistence
- REST API
- Multi-tenancy
- Reference CLI

## Possible Future Enhancements

- Add a concept of "fuel" to constrain CPU usage

## Quick Start

## Architecture

## Host Functions

## Examples

## REST API Reference
