---
sidebar_position: 2
---

# Data storage

## `upsilon-data`

This crate provides the `DataClient` trait, which is implemented for a few
different "data backends", but also for the cache.

It also provides the `DataClientMasterHolder` struct, which is `.manage()`d by
Rocket, and is used to get a `DataQueryMaster` to use in the handlers, which is
basically a nice wrapper over the "raw" interface provided by `QueryImpl`, the
implementors of which do the actual work.

## `upsilon-data-cache-inmemory`

The cache is a special data client, which caches the results of the other data
clients, and if the result of a query is already in the cache, it will return
the cached result, instead of querying the data backend it is "wrapping".

## `upsilon-data-inmemory`

This is a data backend, which stores all the data in memory, and is used for
testing mostly.
